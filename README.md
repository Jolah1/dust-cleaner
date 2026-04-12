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

Example of scanning for dust UTXOs and performing a sweep:

[![asciicast](https://asciinema.org/a/cbuBIDQR7o8jMv9n.svg)](https://asciinema.org/a/cbuBIDQR7o8jMv9n)

---

## What is a Dust Attack?

A dust attack is a privacy attack where an adversary sends tiny amounts of Bitcoin (called "dust") to your wallet addresses. When you later spend those UTXOs alongside your real funds, the attacker can track the transaction graph to cluster your addresses and de-anonymize your wallet.

Dust amounts vary by script type — too small to spend economically on their own, but large enough to act as a tracking tag.

**Further reading:**
- [Dust attack explained](https://www.investopedia.com/terms/d/dusting-attack.asp)
- [Disposing of dust attack UTXOs — Delving Bitcoin](https://delvingbitcoin.org/t/disposing-of-dust-attack-utxos/2215/)
- [BIP174 — Partially Signed Bitcoin Transactions](https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki)

---

## What This Tool Does

1. Connects to your Bitcoin Core node via RPC
2. Scans your wallet for UTXOs below the dust threshold
3. Classifies UTXOs using **per-script-type thresholds** (or a custom threshold you provide)
4. Shows a dry-run preview of the sweep before committing
5. Builds a PSBT that sweeps all dust UTXOs in a single transaction
6. Burn sweeps via OP_RETURN

---

## Dust Thresholds

By default, dust-cleaner uses Bitcoin-accurate per-script-type thresholds based on the byte cost of spending each input type:

| Script type | Input size  | Dust threshold |
|-------------|-------------|----------------|
| P2PKH       | 148 vbytes  | 546 sats       |
| P2WPKH      | 68 vbytes   | 294 sats       |
| P2TR        | 58 vbytes   | 294 sats       |
| P2SH        | 91 vbytes   | 540 sats       |

You can override these with `--threshold` or `DUST_THRESHOLD`.

---

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (1.70 or later)
- [Bitcoin Core](https://bitcoincore.org/en/download/) node (running and synced)

### Build from source

```bash
git clone https://github.com/Jolah1/dust-cleaner.git
cd dust-cleaner
cargo build --release
```

The binary will be at `target/release/dust-cleaner`.

---

## Usage

### Set credentials once per session (recommended)

```bash
export DUST_RPC_URL=http://127.0.0.1:18443
export DUST_RPC_USER=user
export DUST_RPC_PASS=pass

dust-cleaner scan
dust-cleaner sweep --dry-run
dust-cleaner sweep --method op-return
```

---

### Scan for dust UTXOs

```bash
dust-cleaner scan
```

Example output:
```
Found 6 total UTXOs (threshold: per-script-type (P2PKH:546, P2WPKH:294, P2TR:294, P2SH:540))

⚠️  DUST UTXOs (3 found):
   500 sats | 3a8c360f...:0 | P2WPKH | bcrt1q60z...
   300 sats | 87ef9f5b...:1 | P2WPKH | bcrt1qdpl...
   800 sats | 38942b1e...:1 | P2WPKH | bcrt1qr0e...

✅ CLEAN UTXOs (3 found):
   4999999860 sats | 81452410...:0
   4999975540 sats | 38942b1e...:0
   5000000000 sats | f200404e...:0

─────────────────────────────────────────
📊 Summary
   Total UTXOs:    6
   Dust UTXOs:     3 (1600 sats)
   Clean UTXOs:    3 (14999975400 sats)
   Threshold:      per-script-type

   💡 Run 'sweep' to consolidate dust into a single UTXO
─────────────────────────────────────────
```

---

### Dry run — preview sweep without creating a PSBT

```bash
dust-cleaner sweep --dry-run
```

Example output:
```
Found 3 dust UTXOs to sweep:
   500 sats | 3a8c360f...:0
   300 sats | 87ef9f5b...:1
   800 sats | 38942b1e...:1

🔍 Dry Run — no PSBT created

   Method:            Consolidate
   Dust inputs:       3
   Total dust:        1600 sats
   Funder UTXO:       5000000000 sats
   Estimated fee:     626 sats
   Estimated output:  5000000974 sats

   Run without --dry-run to create the PSBT.
```

---

### Sweep — consolidate dust to a fresh address

```bash
dust-cleaner sweep
```

Example output:
```
📎 Method: consolidate — dust swept to fresh address

📊 Sweep Summary:
   Dust inputs:  3
   Total dust:   1600 sats

🧹 Sweep PSBT (base64):
cHNidP8BA...

💡 Next steps:
   Inspect: bitcoin-cli decodepsbt <psbt>
   Sign:    bitcoin-cli walletprocesspsbt <psbt>
   Send:    bitcoin-cli sendrawtransaction <hex>
```

---

### Sweep — OP_RETURN (burn dust to miner fees)

The most private option — dust is burned entirely to miner fees via an
OP_RETURN output containing `"ash"` (ashes to ashes, dust to dust 🪦).
No consolidation output, no address linkage.

```bash
dust-cleaner sweep --method op-return
```

Example output:
```
🔥 Method: op-return — dust burned to miner fees
   Output: OP_RETURN ("ash" — ashes to ashes, dust to dust)

📊 Sweep Summary:
   Dust inputs:  3
   Total dust:   1600 sats

🧹 Sweep PSBT (base64):
cHNidP8BA...
```

---

### Custom threshold

```bash
# Override with a custom threshold in sats
dust-cleaner --threshold 1000 scan

# Using env var instead
export DUST_THRESHOLD=1000
dust-cleaner scan

# Custom RPC URL (mainnet default port)
dust-cleaner --rpc-url http://127.0.0.1:8332 scan
```

---

## Configuration

### CLI Flags and Environment Variables

| Flag | Environment Variable | Default | Description |
|------|---------------------|---------|-------------|
| `--rpc-url` | `DUST_RPC_URL` | `http://127.0.0.1:18443` | Bitcoin Core RPC URL |
| `--rpc-user` | `DUST_RPC_USER` | required | RPC username |
| `--rpc-pass` | `DUST_RPC_PASS` | required | RPC password |
| `--threshold` | `DUST_THRESHOLD` | per-script-type | Custom dust threshold in sats |

CLI flags always override environment variables when explicitly provided.

---

## Testing

Run unit tests:
```bash
cargo test
```

Currently 15 tests covering:
- Dust detection with default and custom thresholds
- Script type detection (P2PKH, P2WPKH, P2TR, P2SH)
- Per-type threshold values
- UTXO classification with edge cases (empty, all-dust, all-clean)
- Smart threshold with user override

### Test on regtest

Set up a regtest node and simulate a dust attack:

```bash
# Create a separate regtest config
mkdir -p ~/.bitcoin/regtest-dev
cat > ~/.bitcoin/regtest-dev/bitcoin.conf << EOF
regtest=1
fallbackfee=0.0001
[regtest]
rpcuser=user
rpcpassword=pass
rpcport=18443
daemon=1
server=1
EOF

# Start the node
bitcoind -conf=$HOME/.bitcoin/regtest-dev/bitcoin.conf \
         -datadir=$HOME/.bitcoin/regtest-dev

# Create wallet and fund it
bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass createwallet "testwallet"
ADDRESS=$(bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass getnewaddress)
bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass generatetoaddress 101 $ADDRESS

# Simulate dust attack
DUST1=$(bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass getnewaddress)
DUST2=$(bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass getnewaddress)
bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass sendtoaddress $DUST1 0.000005
bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass sendtoaddress $DUST2 0.000003
bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass generatetoaddress 1 $ADDRESS

# Set env vars and run
export DUST_RPC_USER=user
export DUST_RPC_PASS=pass
dust-cleaner scan
dust-cleaner sweep --dry-run
dust-cleaner sweep
dust-cleaner sweep --method op-return
```

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
│   ├── psbt_builder.rs  # PSBT construction and dry-run
│   └── types.rs         # Owned Utxo type for testing
├── docs/
│   └── design.md        # Architecture and design decisions
├── JOURNAL.md           # Development journal
└── README.md
```

---

## Resources

- [BIP174 — Partially Signed Bitcoin Transactions](https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki)
- [BIP370 — PSBTv2](https://github.com/bitcoin/bips/blob/master/bip-0370.mediawiki)
- [Bitcoin Core RPC documentation](https://developer.bitcoin.org/reference/rpc/)
- [rust-bitcoin crate](https://docs.rs/bitcoin/latest/bitcoin/)
- [bitcoincore-rpc crate](https://docs.rs/bitcoincore-rpc/latest/bitcoincore_rpc/)
- [Disposing of dust attack UTXOs — Delving Bitcoin](https://delvingbitcoin.org/t/disposing-of-dust-attack-utxos/2215/)

---

## License

MIT