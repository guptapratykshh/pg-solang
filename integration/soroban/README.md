# Soroban Integration Tests

This directory contains integration tests for Solang contracts targeting the Stellar Soroban network.

## Quick Start

### Prerequisites

1. **Solang with Soroban Support**
   ```bash
   # Build Solang from source with Soroban support
   git clone https://github.com/hyperledger-labs/solang.git
   cd solang
   cargo build --release
   ```

2. **Node.js and npm**
   ```bash
   # Install dependencies
   npm install
   ```

3. **Soroban CLI**
   ```bash
   # Install Soroban CLI
   curl -sSf https://soroban.stellar.org/install | sh
   ```

### Running Tests

1. **Build Contracts**
   ```bash
   npm run build
   ```

2. **Deploy Contracts**
   ```bash
   npm run setup
   ```

3. **Run All Tests**
   ```bash
   npm test
   ```

4. **Run Specific Test**
   ```bash
   npm test -- stellar_asset.spec.js
   ```

## Available Tests

### Stellar Asset Contract (`stellar_asset.spec.js`)

A comprehensive ERC20-like token contract with full integration tests.

**Features:**
- Token minting and burning
- Transfer operations
- Allowance management
- Balance tracking
- Authorization controls

**Test Coverage:**
- ✅ Contract deployment
- ✅ Metadata verification
- ✅ Token operations
- ✅ Allowance system
- ✅ Error handling

### Other Tests

- **Counter Contract** (`counter.spec.js`) - Simple counter functionality
- **Cross Contract Calls** (`cross_contract.spec.js`) - Contract interaction tests
- **Auth Framework** (`auth_framework.spec.js`) - Authorization testing
- **Storage Types** (`storage_types.spec.js`) - Data storage testing
- **Runtime Error** (`runtime_error.spec.js`) - Error handling tests

## Test Structure

Each test follows this pattern:

```javascript
describe('Contract Name', () => {
  let keypair, contract, server;

  before(async () => {
    // Setup: Read keys, deploy contracts, initialize
  });

  it('should perform specific operation', async () => {
    // Test: Call contract functions, verify results
  });
});
```

## Configuration

### Network Settings

Tests run against Stellar testnet by default:
- **RPC URL**: `https://soroban-testnet.stellar.org:443`
- **Network**: Testnet
- **Passphrase**: "Test SDF Network ; September 2015"

### Key Management

Tests use the `alice.txt` file for the test account:
- Generated automatically by `setup.js`
- Contains the secret key for test transactions
- **⚠️ Never commit real keys to version control**

## Troubleshooting

### Common Issues

1. **"Soroban target not available"**
   ```bash
   # Ensure you have the latest Solang with Soroban support
   git pull origin main
   cargo build --release
   ```

2. **"Contract deployment failed"**
   ```bash
   # Check testnet balance
   soroban network add testnet
   soroban keys generate alice
   # Fund the account with testnet XLM
   ```

3. **"Network connection failed"**
   ```bash
   # Verify network connectivity
   curl https://soroban-testnet.stellar.org:443
   ```

### Debug Mode

Enable verbose logging:
```bash
DEBUG=* npm test
```

### Manual Contract Deployment

If automatic deployment fails:
```bash
# Compile contract
solang compile stellar_asset.sol --target soroban

# Deploy manually
soroban contract deploy --wasm stellar_asset.wasm --network testnet

# Save contract ID
echo "CONTRACT_ID" > .soroban/contract-ids/stellar_asset.txt
```

## Development

### Adding New Tests

1. Create contract file (`.sol`)
2. Create test file (`.spec.js`)
3. Update `package.json` build script if needed
4. Add deployment to `setup.js`

### Test Helpers

Use the provided helper functions:
- `call_contract_function()` - Execute contract calls
- `extractLogEvent()` - Parse contract events

### Best Practices

- Use descriptive test names
- Test both success and failure cases
- Verify state changes after operations
- Clean up test data when possible
- Use proper error handling

## Contributing

When adding new tests:

1. Follow existing patterns
2. Include comprehensive documentation
3. Test edge cases and error conditions
4. Ensure proper cleanup
5. Update this README if needed

## Resources

- [Soroban Documentation](https://soroban.stellar.org/)
- [Stellar SDK](https://stellar.github.io/js-stellar-sdk/)
- [Solang Documentation](https://solang.readthedocs.io/)
- [Stellar Testnet](https://laboratory.stellar.org/) 