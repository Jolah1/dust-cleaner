# dust-cleaner

A Bitcoin CLI tool that detects dust attack UTXOs in your wallet and sweeps them using PSBTs.

## What is a Dust Attack?

A dust attack is a privacy attack where an adversary sends tiny amounts of Bitcoin (called "dust") to your wallet addresses. When you later spend those UTXOs, the attacker can track the transaction graph to cluster your addresses and de-anonymize your wallet.

Dust amounts are typically below 1000 satoshis — too small to spend economically on their own, but enough to act as a tracking tag.

## What This Tool Does

1. Connects to your Bitcoin Core node via RPC
2. Scans your wallet for UTXOs below the dust threshold (default: 1000 sats)
3. Classifies UTXOs as dust or clean
4. Builds a PSBT that sweeps all dust UTXOs in a single transaction
5. Consolidates dust into a fresh address, removing the attacker's tracking tags

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (1.70 or later)
- Bitcoin Core node (running and synced)

### Build from source
```bash
git clone https://github.com/<your-username>/dust-cleaner.git
cd dust-cleaner
cargo build --release
```

The binary will be at `target/release/dust-cleaner`.

## Usage

### Scan for dust UTXOs
```bash
dust-cleaner --rpc-user <user> --rpc-pass <pass> scan
```

Example output:
```
Found 9 total UTXOs

⚠️  DUST UTXOs (7 found):
   500 sats | 3a8c360f...0:
   300 sats | 87ef9f5b...1
   ...

✅ CLEAN UTXOs (2 found):
   4999987130 sats | aa66b50e...0
   5000000000 sats | 78ff0fed...0
```

### Sweep dust UTXOs
```bash
dust-cleaner --rpc-user <user> --rpc-pass <pass> sweep
```

Example output:
```
Found 7 dust UTXOs to sweep:
   500 sats | 3a8c360f...0
   ...

📊 Sweep Summary:
   Dust inputs:  7
   Total dust:   3000 sats

🧹 Sweep PSBT (base64):
cHNidP8BA...

💡 Next steps:
   Inspect: bitcoin-cli decodepsbt <psbt>
   Sign:    bitcoin-cli walletprocesspsbt <psbt>
   Send:    bitcoin-cli sendrawtransaction <hex>
```

### Custom RPC URL
```bash
dust-cleaner --rpc-url http://127.0.0.1:8332 --rpc-user <user> --rpc-pass <pass> scan
```

## Configuration

| Flag | Default | Description |
|------|---------|-------------|
| `--rpc-url` | `http://127.0.0.1:18443` | Bitcoin Core RPC URL |
| `--rpc-user` | required | RPC username |
| `--rpc-pass` | required | RPC password |

## Testing

Run unit tests:
```bash
cargo test
```

To test on regtest, simulate a dust attack:
```bash
# Start regtest node
bitcoind -regtest -daemon

# Send dust to your own addresses
bitcoin-cli -regtest sendtoaddress <your_address> 0.000005
bitcoin-cli -regtest sendtoaddress <your_address> 0.000003

# Mine a block to confirm
bitcoin-cli -regtest generatetoaddress 1 <your_address>

# Run the tool
dust-cleaner --rpc-user user --rpc-pass pass scan
dust-cleaner --rpc-user user --rpc-pass pass sweep
```

## Project Structure
```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Public module interface
├── cli.rs           # CLI argument definitions
├── rpc.rs           # Bitcoin Core RPC connection
├── scanner.rs       # UTXO fetching
├── analyzer.rs      # Dust detection and classification
└── psbt_builder.rs  # PSBT construction
```

## License

MIT