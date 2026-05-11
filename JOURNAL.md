# dust-cleaner — Development Journal

## Project Overview
A Bitcoin CLI tool that detects dust attack UTXOs in a Bitcoin Core wallet
and sweeps them safely using PSBTs (BIP174).

---

## Month 1

### Week 1 — Research & Concepts

**Goals:** Understand the problem space before writing any code.

**What I studied:**
- Dust attacks: how adversaries send tiny amounts of BTC to wallet addresses
  to track address clusters when the victim spends those UTXOs alongside
  real funds, breaking pseudonymity
- Dust thresholds: why they differ per script type based on the byte cost
  of spending each input type (P2PKH: 546 sats, P2WPKH: 294 sats, P2TR: 294 sats)
- BIP174 (PSBT): read the full spec focusing on Creator and Updater roles,
  understanding global map, input maps, output maps, and offline signing workflows
- Bitcoin Core RPC: studied listunspent, walletcreatefundedpsbt,
  walletprocesspsbt, finalizepsbt, sendrawtransaction
- Rust basics: ownership, modules, error handling with anyhow, CLI with clap

**Key insight:** Dust UTXOs cannot be swept alone — they're too small to cover
fees. A clean UTXO must fund the transaction, with dust UTXOs included as
additional inputs.

**Resources read:**
- BIP174: https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki
- Bitcoin Core RPC docs: https://developer.bitcoin.org/reference/rpc/
- Dust attack: https://www.investopedia.com/terms/d/dusting-attack.asp
- rust-bitcoin docs: https://docs.rs/bitcoin/latest/bitcoin/
- bitcoincore-rpc crate: https://docs.rs/bitcoincore-rpc/latest/bitcoincore_rpc/
- Delving Bitcoin: https://delvingbitcoin.org/t/disposing-of-dust-attack-utxos/2215/

---

### Week 2 — Environment Setup & First Working Code

**Goals:** Get Bitcoin Core running in regtest, connect Rust to it, print UTXOs.

**Regtest config setup:**
Created a separate regtest config to avoid conflicting with existing signet node:

```
~/.bitcoin/regtest-dev/bitcoin.conf
regtest=1
fallbackfee=0.0001
txindex=1
daemon=1
server=1

[regtest]
rpcuser=
rpcpassword=
rpcport=18443
```

Start node:
```bash
bitcoind -conf=$HOME/.bitcoin/regtest-dev/bitcoin.conf \
         -datadir=$HOME/.bitcoin/regtest-dev
```

**First milestone** — printing UTXOs to terminal:
```rust
let utxos = client.list_unspent(None, None, None, None, None)?;
for utxo in utxos {
    println!("{} sats | {}:{}", utxo.amount.to_sat(), utxo.txid, utxo.vout);
}
```

**Problems hit:**
- `Invalid combination of -regtest, -signet` — existing bitcoin.conf had
  signet=1. Solved by using separate config file with -conf flag
- `rpcport only applies in [regtest] section` — fixed by moving rpcport
  under a [regtest] section header
- `Transaction amount too small` — 100 sats is below Bitcoin Core's send
  minimum, used 300 sats minimum instead

**What I learned:**
- Bitcoin Core only allows one network mode at a time
- Config sections like [regtest] scope settings to specific networks
- Coinbase UTXOs need 100 confirmations before they can be spent

---

### Week 3 — Dust Detection & Project Structure

**Goals:** Build classification logic and set up proper module structure.

**Architecture decision:** Use both lib.rs and separate module files:

```
src/
├── main.rs          # CLI entry point only
├── lib.rs           # public module interface
├── cli.rs           # clap CLI definitions
├── rpc.rs           # Bitcoin Core RPC connection
├── scanner.rs       # UTXO fetching
├── analyzer.rs      # dust detection and classification
├── psbt_builder.rs  # PSBT construction
└── types.rs         # owned Utxo type for testing
```

**CLI design decision:** Use CLI arguments for credentials, not a .env file.
Real Bitcoin tools like bitcoin-cli work this way. Credentials never touch
the filesystem.

**First commit pushed:** Basic scan working, UTXOs printing, clean/dust
separation visible.

---

### Week 4 — PSBT Construction

**Goals:** Build the sweep command that creates a valid PSBT.

**The sweep problem:** Dust UTXOs total only ~3000 sats — not enough to
cover fees. walletcreatefundedpsbt rejected with "transaction amount too small".

**Failed approaches:**
1. Pass only dust UTXOs as inputs → rejected, too small to cover fees
2. Pass empty inputs and rely on coin selection → wallet ignored dust UTXOs
3. Pass dust inputs with "inputs" option → not a valid parameter

**Solution:** Use largest clean UTXO as funder (first input), add all dust
UTXOs as mandatory inputs, use subtractFeeFromOutputs for automatic fee calc.

**First successful sweep confirmed on regtest:**
```
c9bceda90c250fddad5348649de5a36fcfcb7fe081fe721c19da77837b6696fc
```

Verified with decodepsbt — all 8 inputs present (1 funder + 7 dust).

---

## Month 2

### Week 5 — Polish & User Experience

**Graceful error handling:**
```
Error: Could not connect to Bitcoin Core. Is your node running?
Error: No wallet loaded. Run: bitcoin-cli loadwallet <name>
```

**Scan summary added:**
```
📊 Summary
   Total UTXOs:    6
   Dust UTXOs:     3 (1600 sats)
   Clean UTXOs:    3 (14999975400 sats)
   💡 Run 'sweep' to consolidate dust
```

**README and design doc written.**

---

### Week 6 — Testing & Custom Types

**Problem:** Testing classify_utxos required ListUnspentResultEntry from
bitcoincore-rpc — awkward without a live node.

**Solution:** Created owned Utxo type in types.rs and parallel
classify_owned_utxos function. Decouples test logic from the RPC crate.

**15 tests written and passing:**
- is_dust with default and custom thresholds
- Script type detection (P2PKH, P2WPKH, P2TR, P2SH)
- Per-type threshold values
- UTXO classification edge cases
- Smart threshold with user override

---

### Week 7 — Per-Script-Type Thresholds, OP_RETURN, Environment Variables

**Per-script-type thresholds:**
Each script type has a different dust threshold based on its input byte size:

| Script type | Input size | Dust threshold |
|-------------|-----------|----------------|
| P2PKH       | 148 vbytes | 546 sats      |
| P2WPKH      | 68 vbytes  | 294 sats      |
| P2TR        | 58 vbytes  | 294 sats      |
| P2SH        | 91 vbytes  | 540 sats      |

**Interesting discovery:** After switching to per-type thresholds, my 300/500/800
sat UTXOs were no longer classified as dust. They are P2WPKH outputs (threshold:
294 sats) and all above threshold. The flat 1000 sat threshold was overly
conservative.

**OP_RETURN sweep method added:**
Burns dust entirely to miner fees via OP_RETURN "ash" (0x617368) output.
No change output, no address linkage.

**Environment variable support:**
```bash
export DUST_RPC_USER=user
export DUST_RPC_PASS=pass
dust-cleaner scan  # no flags needed
```

Implemented using clap's env feature:
```rust
#[arg(long, env = "DUST_RPC_USER")]
pub rpc_user: String,
```

**CI pipeline added:**
- Test: cargo build + cargo test
- Clippy: cargo clippy -- -D warnings
- Format: cargo fmt --check

---

### Week 8 — Privacy Fix: Per-UTXO Default Sweep

**Problem identified by @haris in BOSS Discord:**
Batching all dust UTXOs into one transaction links their addresses on-chain —
exactly what a dust attack exploits.

```
WRONG (batch):
Input 1: dust from address A ─┐
Input 2: dust from address B ─┼─→ output  ← addresses linked!
Input 3: dust from address C ─┘
```

**Fix implemented:**
Changed default sweep to one transaction per dust UTXO:
```
Tx 1: dust from address A → OP_RETURN  (no link to B or C)
Tx 2: dust from address B → OP_RETURN  (no link to A or C)
Tx 3: dust from address C → OP_RETURN  (no link to A or B)
```

Added `--batch` flag for users who explicitly want the old behavior.
Changed default method from `consolidate` to `op-return`.

**PR merged:** https://github.com/Jolah1/dust-cleaner/pull/10

---

### Week 8 (continued) — ANYONECANPAY|NONE Investigation & Security Finding

**What ANYONECANPAY|NONE would do:**
- Each input signs only itself (ANYONECANPAY)
- Signer commits to no outputs (NONE)
- Miners can batch thousands of dust sweeps permissionlessly
- Maximum blockspace efficiency

**Why we stopped:**
Murch flagged on the bitcoindev mailing list that SIGHASH_NONE|ANYONECANPAY
is unsafe — third parties can steal signed inputs as free fee subsidy.

Reference: https://groups.google.com/g/bitcoindev/c/pr1z3_j8vTc/m/DqMYltO_AAAJ

---

### Week 9 — ANYONECANPAY|ALL Implementation

**Goals:** Implement the safe sighash type per Murch's recommendation.

**Why ALL|ANYONECANPAY is safe:**
- `ANYONECANPAY` — each input signs only itself, miners can add more inputs
- `ALL` — signer commits to all outputs, preventing output modification
- Miners can add inputs to cover fees but cannot steal value or change outputs

**Implementation challenge:**
`walletcreatefundedpsbt` does not support custom sighash types. Two approaches
were attempted:

**Attempt 1 — Manual signing with rust-bitcoin:**
Used `dumpprivkey` to get the private key, then signed manually using
rust-bitcoin's `SighashCache` with `EcdsaSighashType::AllPlusAnyoneCanPay`.

**Failed because:** `dumpprivkey` only works with legacy wallets. Our
`testwallet` is a descriptor wallet (default since Bitcoin Core 23).

**Attempt 2 — signrawtransactionwithwallet (final solution):**
1. Get previous tx scriptPubKey via `getrawtransaction` (requires `txindex=1`)
2. Build raw transaction with `createrawtransaction` — OP_RETURN "ash" output
3. Sign with `signrawtransactionwithwallet` passing `"ALL|ANYONECANPAY"` as
   sighash type
4. Output raw signed hex for user to broadcast

**Transaction structure:**
```
Input:   dust UTXO (signed with SIGHASH_ALL|ANYONECANPAY)
Output:  OP_RETURN "ash" (0 sats) — all dust value goes to miner fees
Fee:     entire dust value
```

**No funder UTXO needed** — the dust value itself is the fee.

**Tested on regtest:**
Swept 6 dust UTXOs (300, 500, 800 sats × 2) in separate transactions.
All confirmed. Wallet showed 0 dust UTXOs after sweep.

**Important note:** Requires `txindex=1` in `bitcoin.conf` for
`getrawtransaction` to find previous transactions.

**Broadcast helper:**
```bash
cargo run -- --threshold 1000 sweep --method anyone-can-pay 2>&1 | \
  grep "Hex:" | awk '{print $2}' | while read hex; do
    bitcoin-cli sendrawtransaction $hex
done
```

**Issue #4 closed:** https://github.com/Jolah1/dust-cleaner/issues/4

---

## Challenges & How I Solved Them

| Challenge | Solution |
|-----------|----------|
| Regtest conflicts with existing signet node | Separate config file and datadir |
| Dust UTXOs too small to fund sweep | Add largest clean UTXO as funder |
| Address<NetworkUnchecked> won't print | Call assume_checked() first |
| Testing without live node | Created owned Utxo type for unit tests |
| Address linking in batch sweep | Changed default to per-UTXO sweep |
| ANYONECANPAY|NONE is unsafe | Stopped, used ALL|ANYONECANPAY instead |
| dumpprivkey fails on descriptor wallets | Used signrawtransactionwithwallet |
| getrawtransaction fails without txindex | Added txindex=1 to bitcoin.conf |

---

## What I Learned

**Bitcoin protocol:**
- How dust attacks work and why they are a privacy threat
- Why dust thresholds differ per script type (byte cost of spending)
- How PSBTs enable offline signing workflows (BIP174)
- PSBT lifecycle: Creator → Updater → Signer → Finalizer → Extractor
- Why SIGHASH_NONE|ANYONECANPAY is unsafe (inputs can be stolen as fee subsidy)
- Why ALL|ANYONECANPAY is safe — outputs are locked, miners can only add inputs
- Why per-UTXO sweeping prevents address linking vs batch sweeping
- Why txindex is needed for getrawtransaction on non-wallet transactions
- Difference between legacy wallets and descriptor wallets in Bitcoin Core

**Rust:**
- Module system: lib.rs as public interface + separate module files
- Error handling with anyhow and custom messages
- CLI design with clap derive macros including env var support
- Writing testable code by owning your types
- Lifetime elision rules (clippy caught needless_lifetimes)
- Using rust-bitcoin for transaction construction and sighash calculation

**Open source:**
- Read mailing lists (bitcoindev) alongside GitHub issues
- Get community feedback before shipping features
- Cross-reference other implementations (ddust) for alignment
- The value of closing issues with detailed explanations

---

## Community Engagement

- Referenced Delving Bitcoin thread throughout
- Received feedback from @haris in BOSS Discord on address linking
- Implemented per-UTXO sweep fix based on that feedback
- Stopped ANYONECANPAY|NONE after Murch's security finding
- Referenced ddust BIP PR: https://github.com/bitcoin/bips/pull/2150

---

---

## Future Improvements

- **Staggered broadcast** — random delays between per-UTXO broadcasts
  to prevent timing correlation
- **BIP329 label export** — tag swept UTXOs in Sparrow-compatible format
- **Hardware wallet support** — export PSBTs for Ledger/Coldcard/Trezor
- **Address clustering heuristics** — score each UTXO by attack likelihood
- **Watch-only wallet support** — scan without a hot wallet
- **Private broadcast** — Bitcoin Core v31 privatebroadcast flag integration
- **Automatic broadcast pipeline** — pipe sweep output directly to sendrawtransaction