// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "soroban")]
pub mod soroban_testcases;

use solang::codegen::Options;
use solang::file_resolver::FileResolver;
use solang::sema::ast::Namespace;
use solang::sema::diagnostics::Diagnostics;
use solang::{compile, Target};
use soroban_sdk::testutils::Logs;
use soroban_sdk::{vec, Address, ConstructorArgs, Env, Symbol, Val};
use std::ffi::OsStr;

// TODO: register accounts, related balances, events, etc.
pub struct SorobanEnv {
    env: Env,
    contracts: Vec<Address>,
    compiler_diagnostics: Diagnostics,
}

pub fn build_solidity<F>(src: &str, configure_env: F) -> SorobanEnv
where
    F: FnOnce(&mut SorobanEnv),
{
    let (wasm_blob, ns) = build_wasm(src);

    SorobanEnv::new_with_contract(wasm_blob, configure_env).insert_diagnostics(ns.diagnostics)
}

fn build_wasm(src: &str) -> (Vec<u8>, Namespace) {
    let tmp_file = OsStr::new("test.sol");
    let mut cache = FileResolver::default();
    cache.set_file_contents(tmp_file.to_str().unwrap(), src.to_string());
    let opt = inkwell::OptimizationLevel::Default;
    let target = Target::Soroban;
    let (wasm, ns) = compile(
        tmp_file,
        &mut cache,
        target,
        &Options {
            opt_level: opt.into(),
            log_runtime_errors: true,
            log_prints: true,
            #[cfg(feature = "wasm_opt")]
            wasm_opt: Some(contract_build::OptimizationPasses::Z),
            soroban_version: None,
            ..Default::default()
        },
        std::vec!["unknown".to_string()],
        "0.0.1",
    );
    assert!(!wasm.is_empty());
    (wasm[0].0.clone(), ns)
}

impl SorobanEnv {
    pub fn new() -> Self {
        Self {
            env: Env::default(),
            contracts: Vec::new(),
            compiler_diagnostics: Diagnostics::default(),
        }
    }

    pub fn insert_diagnostics(mut self, diagnostics: Diagnostics) -> Self {
        self.compiler_diagnostics = diagnostics;
        self
    }

    pub fn new_with_contract<F>(contract_wasm: Vec<u8>, configure_env: F) -> Self
    where
        F: FnOnce(&mut SorobanEnv),
    {
        let mut env = Self::new();
        configure_env(&mut env);

        env.register_contract(contract_wasm);

        env
    }

    pub fn register_contract(&mut self, contract_wasm: Vec<u8>) -> Address {
        // For now, we keep using `register_contract_wasm`. To use `register`, we have to figure
        // out first what to pass for `constructor_args`
        #[allow(deprecated)]
        let addr = self
            .env
            .register_contract_wasm(None, contract_wasm.as_slice());
        self.contracts.push(addr.clone());
        addr
    }

    pub fn invoke_contract(&self, addr: &Address, function_name: &str, args: Vec<Val>) -> Val {
        let func = Symbol::new(&self.env, function_name);
        let mut args_soroban = vec![&self.env];
        for arg in args {
            args_soroban.push_back(arg)
        }
        println!("args: {args_soroban:?}");
        // To avoid running out of fuel
        self.env.cost_estimate().budget().reset_unlimited();
        self.env.invoke_contract(addr, &func, args_soroban)
    }

    /// Invoke a contract and expect an error. Returns the logs.
    pub fn invoke_contract_expect_error(
        &self,
        addr: &Address,
        function_name: &str,
        args: Vec<Val>,
    ) -> Vec<String> {
        let func = Symbol::new(&self.env, function_name);
        let mut args_soroban = vec![&self.env];
        for arg in args {
            args_soroban.push_back(arg)
        }

        let _ = self
            .env
            .try_invoke_contract::<Val, Val>(addr, &func, args_soroban);

        self.env.logs().all()
    }

    pub fn deploy_contract(&mut self, src: &str) -> Address {
        let wasm = build_wasm(src).0;

        let addr = self.register_contract(wasm);

        self.contracts.push(addr.clone());

        addr
    }

    pub fn deploy_contract_with_args<A>(&mut self, src: &str, args: A) -> Address
    where
        A: ConstructorArgs,
    {
        let wasm = build_wasm(src).0;

        let addr = self.env.register(wasm.as_slice(), args);

        self.contracts.push(addr.clone());

        addr
    }
}

impl Default for SorobanEnv {
    fn default() -> Self {
        Self::new()
    }
}
