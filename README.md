# dust-cleaner

A Bitcoin CLI tool that detects dust attack UTXOs in your wallet and sweeps them safely using PSBTs ([BIP174](https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki)).

Connects directly to Bitcoin Core via RPC. No keys touched. No auto-broadcast. You stay in control.

---

## CLI Interface

```
Detect and sweep dust attack UTXOs from your Bitcoin wallet

Usage: dust-cleaner [OPTIONS] --rpc-user <RPC_USER> --rpc-pass <RPC_PASS> <COMMAND>

Commands:
  scan   Scan wallet for dust UTXOs
  sweep  Create a PSBT sweeping all dust UTXOs
  help   Print this message or the help of the given subcommand(s)

Options:
      --rpc-url <RPC_URL>      Bitcoin Core RPC URL [env: DUST_RPC_URL=] [default: http://127.0.0.1:18443]
      --rpc-user <RPC_USER>    Bitcoin Core RPC username [env: DUST_RPC_USER=]
      --rpc-pass <RPC_PASS>    Bitcoin Core RPC password [env: DUST_RPC_PASS=]
      --threshold <THRESHOLD>  Dust threshold in sats [env: DUST_THRESHOLD=]
  -h, --help                   Print help
  -V, --version                Print version
```

---

## Demo

[![asciicast](https://asciinema.org/a/rIVGJRHVRasoH6u7.svg)](https://asciinema.org/a/rIVGJRHVRasoH6u7)

---

## What is a Dust Attack?

A dust attack is a privacy attack where an adversary sends tiny amounts of Bitcoin (called "dust") to your wallet addresses. When you later spend those UTXOs alongside your real funds, the attacker can track the transaction graph to cluster your addresses and de-anonymize your wallet.

Dust amounts vary by script type — too small to spend economically on their own, but large enough to act as a tracking tag.

**Further reading:**
- [Dust attack explained](https://www.investopedia.com/terms/d/dusting-attack.asp)
- [Disposing of dust attack UTXOs — Delving Bitcoin](https://delvingbitcoin.org/t/disposing-of-dust-attack-utxos/2215/)
- [Dust UTXO Disposal Protocol — BIP draft](https://github.com/bitcoin/bips/pull/2150)
- [BIP174 — Partially Signed Bitcoin Transactions](https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki)

---

## What This Tool Does

1. Connects to your Bitcoin Core node via RPC
2. Scans your wallet for UTXOs below the dust threshold
3. Classifies UTXOs using **per-script-type thresholds** (or a custom threshold)
4. Shows a dry-run preview before committing
5. By default, sweeps **one UTXO per transaction** — no address linking
6. Three sweep methods with increasing privacy levels

---

## Dust Thresholds

By default, dust-cleaner uses Bitcoin-accurate per-script-type thresholds:

| Script type | Input size  | Dust threshold |
|-------------|-------------|----------------|
| P2PKH       | 148 vbytes  | 546 sats       |
| P2WPKH      | 68 vbytes   | 294 sats       |
| P2TR        | 58 vbytes   | 294 sats       |
| P2SH        | 91 vbytes   | 540 sats       |

Override with `--threshold` or `DUST_THRESHOLD`.

---

## Sweep Methods

| Method | Command | Privacy | Address linking | Output |
|--------|---------|---------|----------------|--------|
| Per-UTXO OP_RETURN | `sweep` (default) | ✅ highest | ❌ none | OP_RETURN "ash" |
| ANYONECANPAY\|ALL | `sweep --method anyone-can-pay` | ✅ highest + miner batchable | ❌ none | OP_RETURN "ash" |
| Per-UTXO consolidate | `sweep --method consolidate` | ✅ high | ❌ none | fresh address |
| Batch OP_RETURN | `sweep --batch --method op-return` | ⚠️ medium | ✅ yes | OP_RETURN "ash" |
| Batch consolidate | `sweep --batch --method consolidate` | ❌ lowest | ✅ yes | fresh address |

---

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (1.70 or later)
- [Bitcoin Core](https://bitcoincore.org/en/download/) node (running and synced)
- `txindex=1` in your `bitcoin.conf` (required for `--method anyone-can-pay`)

### Build from source

```bash
git clone https://github.com/Jolah1/dust-cleaner.git
cd dust-cleaner
cargo build --release
```

Binary at `target/release/dust-cleaner`.

---

## Quick Start: Regtest Setup

### 1. Create a regtest config

```bash
mkdir -p ~/.bitcoin/regtest-dev
cat > ~/.bitcoin/regtest-dev/bitcoin.conf << EOF
regtest=1
fallbackfee=0.0001
txindex=1
daemon=1
server=1

[regtest]
rpcuser=user
rpcpassword=pass
rpcport=18443
EOF
```

### 2. Start Bitcoin Core

```bash
bitcoind -conf=$HOME/.bitcoin/regtest-dev/bitcoin.conf \
         -datadir=$HOME/.bitcoin/regtest-dev
```

### 3. Create and fund a wallet

```bash
bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass createwallet "testwallet"

ADDRESS=$(bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass getnewaddress)
bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass generatetoaddress 101 $ADDRESS
```

### 4. Simulate a dust attack

```bash
DUST1=$(bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass getnewaddress)
DUST2=$(bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass getnewaddress)
DUST3=$(bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass getnewaddress)

bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass sendtoaddress $DUST1 0.000005
bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass sendtoaddress $DUST2 0.000003
bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass sendtoaddress $DUST3 0.000008

bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass generatetoaddress 1 $ADDRESS
```

### 5. Set credentials and run

```bash
export DUST_RPC_USER=user
export DUST_RPC_PASS=pass

# Scan — detect dust
dust-cleaner --threshold 1000 scan

# Preview sweep
dust-cleaner --threshold 1000 sweep --dry-run

# Sweep with ANYONECANPAY|ALL (most private)
dust-cleaner --threshold 1000 sweep --method anyone-can-pay
```

### 6. Broadcast each signed transaction

```bash
# The sweep command outputs raw signed hex for each UTXO
# Broadcast each one:
bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass sendrawtransaction <hex>

# Confirm
bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass generatetoaddress 1 $(bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass getnewaddress)

# Verify wallet is clean
dust-cleaner --threshold 1000 scan
```

---

## Usage

### Set credentials once per session

```bash
export DUST_RPC_URL=http://127.0.0.1:18443
export DUST_RPC_USER=user
export DUST_RPC_PASS=pass
```

### Scan for dust UTXOs

```bash
dust-cleaner scan
```

Output:
```
Found 6 total UTXOs (threshold: per-script-type (P2PKH:546, P2WPKH:294, P2TR:294, P2SH:540))

⚠️  DUST UTXOs (3 found):
   500 sats | 3a8c360f...:0 | P2WPKH | bcrt1q60z...
   300 sats | 87ef9f5b...:1 | P2WPKH | bcrt1qdpl...
   800 sats | 38942b1e...:1 | P2WPKH | bcrt1qr0e...

✅ CLEAN UTXOs (3 found):
   4999999860 sats | 81452410...:0
   ...

📊 Summary
   Dust UTXOs:  3 (1600 sats)
   Clean UTXOs: 3 (14999975400 sats)
```

### Dry run

```bash
dust-cleaner sweep --dry-run
```

### Sweep — ANYONECANPAY|ALL (most private, miner batchable)

```bash
dust-cleaner --threshold 1000 sweep --method anyone-can-pay
```

Output:
```
⚡ Method: anyonecanpay|all — maximum privacy
   Sighash: SIGHASH_ALL | SIGHASH_ANYONECANPAY
   Each input signed independently — no address linking
   Outputs locked — miners can add inputs but not change outputs
   Miners can batch these transactions permissionlessly

📊 Generated 3 signed transactions:

─── Tx 1 of 3 ───
   Address: bcrt1q62sx9...
   Dust:    300 sats → miner fees
   Hex:     02000000...

─── Tx 2 of 3 ───
   Address: bcrt1q4x2qz...
   Dust:    500 sats → miner fees
   Hex:     02000000...

─── Tx 3 of 3 ───
   Address: bcrt1qyk3sa...
   Dust:    800 sats → miner fees
   Hex:     02000000...

💡 Broadcast each transaction:
   bitcoin-cli sendrawtransaction <hex>

⚠️  Broadcast at different times to prevent timing correlation.
```

### Sweep — per-UTXO OP_RETURN (default)

```bash
dust-cleaner sweep
```

### Sweep — batch (opt-in, links addresses)

```bash
dust-cleaner sweep --batch

# With privacy warning shown automatically
⚠️  Mode: batch — all dust UTXOs swept in one transaction
   Warning: this links all dust addresses on-chain.
```

### Custom threshold

```bash
dust-cleaner --threshold 1000 scan
export DUST_THRESHOLD=1000
dust-cleaner scan
```

---

## Configuration

| Flag | Environment Variable | Default | Description |
|------|---------------------|---------|-------------|
| `--rpc-url` | `DUST_RPC_URL` | `http://127.0.0.1:18443` | Bitcoin Core RPC URL |
| `--rpc-user` | `DUST_RPC_USER` | required | RPC username |
| `--rpc-pass` | `DUST_RPC_PASS` | required | RPC password |
| `--threshold` | `DUST_THRESHOLD` | per-script-type | Custom dust threshold in sats |

---

## Testing

```bash
cargo test
```

15 tests covering dust detection, script type detection, per-type thresholds,
UTXO classification edge cases, and smart threshold with user override.

---

## Project Structure

```
dust-cleaner/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Public module interface
│   ├── cli.rs           # CLI argument definitions (clap)
│   ├── rpc.rs           # Bitcoin Core RPC connection
│   ├── scanner.rs       # UTXO fetching via list_unspent
│   ├── analyzer.rs      # Dust detection and classification
│   ├── psbt_builder.rs  # PSBT construction, dry-run, ANYONECANPAY|ALL
│   └── types.rs         # Owned Utxo type for testing
├── docs/
│   └── design.md
├── JOURNAL.md
└── README.md
```

---

## Resources

- [BIP174 — Partially Signed Bitcoin Transactions](https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki)
- [BIP143 — Sighash types](https://github.com/bitcoin/bips/blob/master/bip-0143.mediawiki)
- [Dust UTXO Disposal Protocol — BIP draft](https://github.com/bitcoin/bips/pull/2150)
- [Bitcoin Core RPC documentation](https://developer.bitcoin.org/reference/rpc/)
- [rust-bitcoin crate](https://docs.rs/bitcoin/latest/bitcoin/)
- [bitcoincore-rpc crate](https://docs.rs/bitcoincore-rpc/latest/bitcoincore_rpc/)
- [Disposing of dust attack UTXOs — Delving Bitcoin](https://delvingbitcoin.org/t/disposing-of-dust-attack-utxos/2215/)
---

## License

MIT