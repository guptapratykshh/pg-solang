// SPDX-License-Identifier: Apache-2.0

pub(super) mod target;
use crate::codegen::{cfg::ControlFlowGraph, HostFunctions, Options};

use crate::emit::cfg::emit_cfg;
use crate::{emit::Binary, sema::ast};
use funty::Fundamental;
use inkwell::{
    context::Context,
    module::{Linkage, Module},
    types::FunctionType,
};
use soroban_sdk::xdr::{
    Limited, Limits, ScEnvMetaEntry, ScEnvMetaEntryInterfaceVersion, ScSpecEntry,
    ScSpecFunctionInputV0, ScSpecFunctionV0, ScSpecTypeDef, StringM, WriteXdr,
};

const SOROBAN_ENV_INTERFACE_VERSION: ScEnvMetaEntryInterfaceVersion =
    ScEnvMetaEntryInterfaceVersion {
        protocol: 22,
        pre_release: 0,
    };

impl HostFunctions {
    pub fn function_signature<'b>(&self, bin: &Binary<'b>) -> FunctionType<'b> {
        let ty = bin.context.i64_type();
        match self {
            HostFunctions::PutContractData => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into(), ty.into()], false),
            HostFunctions::GetContractData => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into()], false),
            HostFunctions::HasContractData => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into()], false),
            // https://github.com/stellar/stellar-protocol/blob/2fdc77302715bc4a31a784aef1a797d466965024/core/cap-0046-03.md#ledger-host-functions-mod-l
            // ;; If the entry's TTL is below `threshold` ledgers, extend `live_until_ledger_seq` such that TTL == `extend_to`, where TTL is defined as live_until_ledger_seq - current ledger.
            // (func $extend_contract_data_ttl (param $k_val i64) (param $t_storage_type i64) (param $threshold_u32_val i64) (param $extend_to_u32_val i64) (result i64))
            HostFunctions::ExtendContractDataTtl => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into(), ty.into(), ty.into()], false),
            // ;; If the TTL for the current contract instance and code (if applicable) is below `threshold` ledgers, extend `live_until_ledger_seq` such that TTL == `extend_to`, where TTL is defined as live_until_ledger_seq - current ledger.
            // (func $extend_current_contract_instance_and_code_ttl (param $threshold_u32_val i64) (param $extend_to_u32_val i64) (result i64))
            HostFunctions::ExtendCurrentContractInstanceAndCodeTtl => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into()], false),
            HostFunctions::LogFromLinearMemory => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into(), ty.into(), ty.into()], false),
            HostFunctions::SymbolNewFromLinearMemory => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into()], false),
            HostFunctions::VectorNew => bin.context.i64_type().fn_type(&[], false),
            HostFunctions::Call => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into(), ty.into()], false),
            HostFunctions::VectorNewFromLinearMemory => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into()], false),
            HostFunctions::ObjToU64 => bin.context.i64_type().fn_type(&[ty.into()], false),
            HostFunctions::ObjFromU64 => bin.context.i64_type().fn_type(&[ty.into()], false),
            HostFunctions::RequireAuth => bin.context.i64_type().fn_type(&[ty.into()], false),
            HostFunctions::AuthAsCurrContract => {
                bin.context.i64_type().fn_type(&[ty.into()], false)
            }
            HostFunctions::MapNewFromLinearMemory => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into(), ty.into()], false),

            HostFunctions::MapNew => bin.context.i64_type().fn_type(&[], false),

            HostFunctions::MapPut => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into(), ty.into()], false),

            HostFunctions::VecPushBack => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into()], false),

            HostFunctions::StringNewFromLinearMemory => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into()], false),
            HostFunctions::StrKeyToAddr => bin.context.i64_type().fn_type(&[ty.into()], false),
            HostFunctions::GetCurrentContractAddress => bin.context.i64_type().fn_type(&[], false),
            HostFunctions::ObjToI128Lo64 => bin.context.i64_type().fn_type(&[ty.into()], false),
            HostFunctions::ObjToI128Hi64 => bin.context.i64_type().fn_type(&[ty.into()], false),
            HostFunctions::ObjToU128Lo64 => bin.context.i64_type().fn_type(&[ty.into()], false),
            HostFunctions::ObjToU128Hi64 => bin.context.i64_type().fn_type(&[ty.into()], false),
            HostFunctions::ObjFromI128Pieces => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into()], false),
            HostFunctions::ObjFromU128Pieces => bin
                .context
                .i64_type()
                .fn_type(&[ty.into(), ty.into()], false),
        }
    }
}

pub struct SorobanTarget;

impl SorobanTarget {
    pub fn build<'a>(
        context: &'a Context,
        std_lib: &Module<'a>,
        contract: &'a ast::Contract,
        ns: &'a ast::Namespace,
        opt: &'a Options,
        contract_no: usize,
    ) -> Binary<'a> {
        let filename = ns.files[contract.loc.file_no()].file_name();
        let mut bin = Binary::new(
            context,
            ns,
            &contract.id.name,
            &filename,
            opt,
            std_lib,
            None,
        );

        let mut export_list = Vec::new();
        Self::declare_externals(&mut bin);
        Self::emit_functions_with_spec(contract, &mut bin, context, contract_no, &mut export_list);
        bin.internalize(export_list.as_slice());

        //Self::emit_initializer(&mut binary, ns, contract.constructors(ns).first());

        Self::emit_env_meta_entries(context, &mut bin, opt);

        bin
    }

    // In Soroban, the public functions specifications is embeded in the contract binary.
    // for each function, emit both the function spec entry and the function body.
    fn emit_functions_with_spec<'a>(
        contract: &'a ast::Contract,
        bin: &mut Binary<'a>,
        context: &'a Context,
        _contract_no: usize,
        export_list: &mut Vec<&'a str>,
    ) {
        let mut defines = Vec::new();

        for (cfg_no, cfg) in contract.cfg.iter().enumerate() {
            let ftype = bin.function_type(
                &cfg.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
                &cfg.returns.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
            );

            // For each function, determine the name and the linkage
            // Soroban has no dispatcher, so all externally addressable functions are exported and should be named the same as the original function name in the source code.
            // If there are duplicate function names, then the function name in the source is mangled to include the signature.

            // if func is a default constructor, then the function name is the contract name

            let linkage = if cfg.public {
                let name = if cfg.name.contains("::") {
                    // get the third part of the name which is the function name
                    cfg.name.split("::").collect::<Vec<&str>>()[2]
                } else {
                    &cfg.name
                };
                Self::emit_function_spec_entry(context, cfg, name.to_string(), bin);
                export_list.push(name);
                Linkage::External
            } else {
                Linkage::Internal
            };

            let func_decl = if let Some(func) = bin.module.get_function(&cfg.name) {
                // must not have a body yet
                assert_eq!(func.get_first_basic_block(), None);

                func
            } else {
                bin.module.add_function(&cfg.name, ftype, Some(linkage))
            };

            bin.functions.insert(cfg_no, func_decl);

            defines.push((func_decl, cfg));
        }

        let init_type = context.i64_type().fn_type(&[], false);
        bin.module
            .add_function("storage_initializer", init_type, None);

        for (func_decl, cfg) in defines {
            emit_cfg(&mut SorobanTarget, bin, contract, cfg, func_decl);
        }
    }

    fn emit_env_meta_entries<'a>(context: &'a Context, bin: &mut Binary<'a>, opt: &'a Options) {
        let mut meta = Limited::new(Vec::new(), Limits::none());
        let soroban_env_interface_version = opt.soroban_version;
        let soroban_env_interface_version = match soroban_env_interface_version {
            Some(version) => ScEnvMetaEntryInterfaceVersion {
                protocol: version.as_u32(),
                pre_release: 0,
            },
            None => SOROBAN_ENV_INTERFACE_VERSION,
        };
        ScEnvMetaEntry::ScEnvMetaKindInterfaceVersion(soroban_env_interface_version)
            .write_xdr(&mut meta)
            .expect("writing env meta interface version to xdr");
        Self::add_custom_section(context, &bin.module, "contractenvmetav0", meta.inner);
    }

    fn emit_function_spec_entry<'a>(
        context: &'a Context,
        cfg: &ControlFlowGraph,
        name: String,
        bin: &mut Binary<'a>,
    ) {
        if cfg.public && !cfg.is_placeholder() {
            // TODO: Emit custom type spec entries
            let mut spec = Limited::new(Vec::new(), Limits::none());
            ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
                name: name
                    .try_into()
                    .unwrap_or_else(|_| panic!("function name {:?} exceeds limit", cfg.name)),
                inputs: cfg
                    .params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| ScSpecFunctionInputV0 {
                        name: p
                            .id
                            .as_ref()
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| i.to_string())
                            .try_into()
                            .expect("function input name exceeds limit"),
                        type_: {
                            let ty = if let ast::Type::Ref(ty) = &p.ty {
                                ty.as_ref()
                            } else {
                                &p.ty
                            };

                            match ty {
                                ast::Type::Uint(32) => ScSpecTypeDef::U32,
                                ast::Type::Int(32) => ScSpecTypeDef::I32,
                                ast::Type::Uint(64) => ScSpecTypeDef::U64,
                                &ast::Type::Int(64) => ScSpecTypeDef::I64,
                                ast::Type::Int(128) => ScSpecTypeDef::I128,
                                ast::Type::Uint(128) => ScSpecTypeDef::U128,
                                ast::Type::Bool => ScSpecTypeDef::Bool,
                                ast::Type::Address(_) => ScSpecTypeDef::Address,
                                ast::Type::Bytes(_) => ScSpecTypeDef::Bytes,
                                ast::Type::String => ScSpecTypeDef::String,
                                _ => panic!("unsupported input type {:?}", p.ty),
                            }
                        }, // TODO: Map type.
                        doc: StringM::default(), // TODO: Add doc.
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .expect("function input count exceeds limit"),
                outputs: cfg
                    .returns
                    .iter()
                    .map(|return_type| {
                        let ret_type = return_type.ty.clone();
                        let ty = if let ast::Type::Ref(ty) = ret_type {
                            *ty
                        } else {
                            ret_type
                        };
                        match ty {
                            ast::Type::Uint(32) => ScSpecTypeDef::U32,
                            ast::Type::Int(32) => ScSpecTypeDef::I32,
                            ast::Type::Uint(64) => ScSpecTypeDef::U64,
                            ast::Type::Int(128) => ScSpecTypeDef::I128,
                            ast::Type::Uint(128) => ScSpecTypeDef::U128,
                            ast::Type::Int(_) => ScSpecTypeDef::I32,
                            ast::Type::Bool => ScSpecTypeDef::Bool,
                            ast::Type::Address(_) => ScSpecTypeDef::Address,
                            ast::Type::Bytes(_) => ScSpecTypeDef::Bytes,
                            ast::Type::String => ScSpecTypeDef::String,
                            ast::Type::Void => ScSpecTypeDef::Void,
                            _ => panic!("unsupported return type {ty:?}"),
                        }
                    }) // TODO: Map type.
                    .collect::<Vec<_>>()
                    .try_into()
                    .expect("function output count exceeds limit"),
                doc: StringM::default(), // TODO: Add doc.
            })
            .write_xdr(&mut spec)
            .unwrap_or_else(|_| panic!("writing spec to xdr for function {}", cfg.name));

            Self::add_custom_section(context, &bin.module, "contractspecv0", spec.inner);
        }
    }

    fn add_custom_section<'a>(
        context: &'a Context,
        module: &Module<'a>,
        name: &'a str,
        value: Vec<u8>,
    ) {
        let value_str = unsafe {
            // TODO: Figure out the right way to generate the LLVM metadata for
            // a slice of bytes.
            String::from_utf8_unchecked(value)
        };

        module
            .add_global_metadata(
                "wasm.custom_sections",
                &context.metadata_node(&[
                    context.metadata_string(name).into(),
                    context.metadata_string(&value_str).into(),
                ]),
            )
            .expect("adding spec as metadata");
    }

    fn declare_externals(bin: &mut Binary) {
        let host_functions = [
            HostFunctions::PutContractData,
            HostFunctions::GetContractData,
            HostFunctions::HasContractData,
            HostFunctions::ExtendContractDataTtl,
            HostFunctions::ExtendCurrentContractInstanceAndCodeTtl,
            HostFunctions::LogFromLinearMemory,
            HostFunctions::SymbolNewFromLinearMemory,
            HostFunctions::VectorNew,
            HostFunctions::Call,
            HostFunctions::VectorNewFromLinearMemory,
            HostFunctions::ObjToU64,
            HostFunctions::ObjFromU64,
            HostFunctions::PutContractData,
            HostFunctions::ObjToI128Lo64,
            HostFunctions::ObjToI128Hi64,
            HostFunctions::ObjToU128Lo64,
            HostFunctions::ObjToU128Hi64,
            HostFunctions::ObjFromI128Pieces,
            HostFunctions::ObjFromU128Pieces,
            HostFunctions::RequireAuth,
            HostFunctions::AuthAsCurrContract,
            HostFunctions::MapNewFromLinearMemory,
            HostFunctions::MapNew,
            HostFunctions::MapPut,
            HostFunctions::VecPushBack,
            HostFunctions::StringNewFromLinearMemory,
            HostFunctions::StrKeyToAddr,
            HostFunctions::GetCurrentContractAddress,
        ];

        for func in &host_functions {
            bin.module.add_function(
                func.name(),
                func.function_signature(bin),
                Some(Linkage::External),
            );
        }
    }
}
