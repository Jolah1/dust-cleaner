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
- BIP174 (PSBT): read the full spec focusing on the Creator and Updater roles,
  understanding the global map, input maps, output maps, and how PSBTs allow
  offline signing workflows
- Bitcoin Core RPC: studied listunspent, walletcreatefundedpsbt, walletprocesspsbt,
  finalizepsbt, sendrawtransaction and how they chain together
- Rust basics: ownership, modules, error handling with anyhow, CLI with clap

**Key insight:** Dust UTXOs cannot be swept alone — they're too small to cover
fees. A clean UTXO must fund the transaction, with dust UTXOs included as
additional inputs. The sweep output goes back to the wallet owner, consolidating
everything into a fresh address.

**Resources read:**
- BIP174: https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki
- Bitcoin Core RPC docs: https://developer.bitcoin.org/reference/rpc/
- Dust attack explainer: https://www.investopedia.com/terms/d/dusting-attack.asp
- rust-bitcoin docs: https://docs.rs/bitcoin/latest/bitcoin/
- bitcoincore-rpc crate: https://docs.rs/bitcoincore-rpc/latest/bitcoincore_rpc/
- Disposing of dust attack UTXOs: https://delvingbitcoin.org/t/disposing-of-dust-attack-utxos/2215/

---

### Week 2 — Environment Setup & First Working Code

**Goals:** Get Bitcoin Core running in regtest, connect Rust to it, print UTXOs.

**What I did:**

Set up a separate regtest config to avoid conflicting with my existing signet
node used for BOSS challenges:

```
~/.bitcoin/regtest-dev/bitcoin.conf
regtest=1
fallbackfee=0.0001
rpcuser=user
rpcpassword=pass
rpcport=18443
daemon=1
server=1
```

Created the Rust project:
```bash
git clone https://github.com/Jolah1/dust-cleaner.git
cd dust-cleaner
cargo init --bin
```

First milestone — printing UTXOs to the terminal:
```rust
let utxos = client.list_unspent(None, None, None, None, None)?;
for utxo in utxos {
    println!("{} sats | {}:{}", utxo.amount.to_sat(), utxo.txid, utxo.vout);
}
```

Simulated dust attacks on regtest by sending small amounts to my own addresses.

**Problems hit:**
- `Invalid combination of -regtest, -signet` — existing bitcoin.conf had signet=1,
  solved by using a separate config file with -conf flag
- `rpcport only applies in [regtest] section` — fixed by moving rpcport under
  a [regtest] section header in the config
- `Transaction amount too small` — 100 sats is below Bitcoin Core's own send
  minimum, used 300 sats minimum instead

**What I learned:**
- Bitcoin Core only allows one network mode at a time
- Config sections like [regtest] scope settings to specific networks
- The coinbase UTXO needs 100 confirmations before it can be spent

---

### Week 3 — Dust Detection & Project Structure

**Goals:** Build classification logic and set up proper module structure.

**Architecture decision:** Use both lib.rs and separate module files.
lib.rs declares and re-exports all modules as the public interface.
Each module has a single responsibility.

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
Real Bitcoin tools like bitcoin-cli itself work this way. Credentials never
touch the filesystem.

**First commit pushed:** Basic scan working, UTXOs printing to terminal,
clean/dust separation visible.

---

### Week 4 — PSBT Construction

**Goals:** Build the sweep command that creates a valid PSBT.

**The sweep problem:** Dust UTXOs total only ~3000 sats. This is not enough
to cover transaction fees. Bitcoin Core's walletcreatefundedpsbt rejected
our first attempts with "transaction amount too small".

**Failed approaches:**
1. Pass only dust UTXOs as inputs → rejected, too small to cover fees
2. Pass empty inputs and rely on coin selection → wallet ignored dust UTXOs
3. Pass dust inputs with "inputs" option in options map → not a valid parameter

**Solution that worked:**
Use the largest clean UTXO as the primary funder (first input), then add
all dust UTXOs as additional mandatory inputs. Set the output amount to the
funder's full value and use subtractFeeFromOutputs so Bitcoin Core calculates
the exact fee automatically.

**First successful sweep txid confirmed on regtest:**
```
c9bceda90c250fddad5348649de5a36fcfcb7fe081fe721c19da77837b6696fc
```

Verified with decodepsbt that all 8 inputs were present (1 funder + 7 dust).

**What I learned:**
- PSBTs allow the Creator role to specify mandatory inputs
- walletcreatefundedpsbt handles fee calculation automatically when
  subtractFeeFromOutputs is set
- The PSBT workflow: create → sign (walletprocesspsbt) → finalize → broadcast

---

## Month 2

### Week 5 — Polish & User Experience

**Goals:** Make the tool production-quality with proper error messages,
summary output, and safe credential handling.

**Graceful error handling added:**

Instead of crashing with raw RPC errors, the tool now shows helpful messages:

```
Error: Could not connect to Bitcoin Core. Is your node running?
Tip: bitcoind -conf=/home/$USER/.bitcoin/regtest-dev/bitcoin.conf ...

Error: No wallet loaded.
Tip: bitcoin-cli -rpcport=<port> -rpcuser=<user> -rpcpassword=<pass> loadwallet <walletname>
```

**Scan summary line added showing:**
- Total UTXOs found
- Dust UTXOs count and total value
- Clean UTXOs count and total value
- Active threshold
- Prompt to run sweep if dust detected

**README and design doc written:**
- README.md: installation, usage examples, configuration table
- docs/design.md: architecture diagram, module responsibilities,
  sweep strategy, security considerations, future improvements

---

### Week 6 — Testing & Custom Types

**Goals:** Add comprehensive tests, create owned types for testability.

**Problem:** Testing classify_utxos required constructing
ListUnspentResultEntry from bitcoincore-rpc — awkward and tightly coupled
to the external crate's internals.

**Solution:** Created our own Utxo type in types.rs and a parallel
classify_owned_utxos function that operates on our type. This decouples
test logic from the RPC crate entirely.

**Tests written (15 total):**
- test_is_dust_default_threshold
- test_is_dust_custom_threshold
- test_is_dust_zero_threshold
- test_classify_owned_utxos_splits_correctly
- test_classify_owned_utxos_all_dust
- test_classify_owned_utxos_all_clean
- test_classify_owned_utxos_empty
- test_classify_owned_utxos_custom_threshold
- test_detect_script_type_p2pkh
- test_detect_script_type_p2sh
- test_detect_script_type_p2wpkh
- test_detect_script_type_p2tr
- test_dust_thresholds_per_type
- test_is_dust_smart_p2wpkh
- test_is_dust_smart_user_override

All 15 passing.

---

### Week 7 — Per-Script-Type Thresholds & --threshold Flag

**Goals:** Replace flat threshold with Bitcoin-accurate per-type thresholds.

**Why thresholds differ per script type:**
The dust threshold is defined as the minimum UTXO value where the fee cost
to spend it is less than the UTXO's value. Since different script types
produce inputs of different byte sizes, their fee costs differ:

| Script type | Input size | Dust threshold |
|-------------|-----------|----------------|
| P2PKH       | 148 vbytes | 546 sats      |
| P2WPKH      | 68 vbytes  | 294 sats      |
| P2TR        | 58 vbytes  | 294 sats      |
| P2SH        | 91 vbytes  | 540 sats      |

**Implementation:**
Detect script type from address prefix:
```rust
pub fn detect_script_type(address: &str) -> ScriptType {
    if address.starts_with("1") { ScriptType::P2PKH }
    else if address.starts_with("3") { ScriptType::P2SH }
    else if address.starts_with("bc1q") || address.starts_with("bcrt1q") { ScriptType::P2WPKH }
    else if address.starts_with("bc1p") || address.starts_with("bcrt1p") { ScriptType::P2TR }
    else { ScriptType::Unknown }
}
```

**Interesting discovery:** After switching to per-script-type thresholds,
my 300/500/800 sat UTXOs were no longer classified as dust. They are P2WPKH
outputs (threshold: 294 sats) and are all above the threshold — meaning
they are economically spendable. The flat 1000 sat threshold was overly
conservative.

**--threshold flag** made optional. When not provided, per-type thresholds
apply automatically. When provided, it overrides all per-type thresholds.

---

### Week 7 (continued) — OP_RETURN Sweep Method

**Goals:** Add OP_RETURN sweep method to burn dust to miner fees.

**Why OP_RETURN:**
Instead of consolidating dust to a new address (which creates a change
output and maintains an on-chain presence), the OP_RETURN method burns
the entire dust value to miner fees. The output contains "ash" (0x617368)
as a nod to "ashes to ashes, dust to dust."

**Implementation:**
```
Inputs:  funder UTXO + dust UTXOs
Outputs: OP_RETURN "ash" (0 sats) + change back to wallet
Fees:    subtracted from change output automatically
```

**Referenced:** https://delvingbitcoin.org/t/disposing-of-dust-attack-utxos/2215/2

---

### Week 7 (continued) — Environment Variables & CI

**Goals:** Make credentials easier to manage, add CI pipeline.

**Environment variable support added:**
Users can now set credentials once per session:
```bash
export DUST_RPC_USER=user
export DUST_RPC_PASS=pass
dust-cleaner scan  # no credentials needed
```

Implemented using clap's `env` feature:
```rust
#[arg(long, env = "DUST_RPC_USER")]
pub rpc_user: String,
```

**CI pipeline added** (.github/workflows/rust.yml):
- Test job: cargo build + cargo test
- Clippy job: cargo clippy -- -D warnings
- Format job: cargo fmt --check

All 3 jobs pass on every push to main.

---

### Week 8 — Privacy Fix: Per-UTXO Default Sweep

**Goals:** Fix the address linking problem identified by @haris in the
BOSS 2026 Discord.

**The problem:**
The original sweep batched all dust UTXOs into a single transaction:
```
Input 1: dust from address A ─┐
Input 2: dust from address B ─┼─→ OP_RETURN
Input 3: dust from address C ─┘
```
This links all three addresses on-chain — exactly what a dust attack exploits.

**The fix:**
Changed the default sweep behavior to create one transaction per dust UTXO:
```
Tx 1: dust from address A → OP_RETURN  (separate)
Tx 2: dust from address B → OP_RETURN  (separate)
Tx 3: dust from address C → OP_RETURN  (separate)
```

Added `--batch` flag for users who explicitly want the old behavior:
```bash
dust-cleaner sweep           # default: per-UTXO, most private
dust-cleaner sweep --batch   # opt-in: batch, faster but links addresses
```

Also changed the default method from `consolidate` to `op-return` since
OP_RETURN is the more privacy-preserving option.

**PR:** https://github.com/Jolah1/dust-cleaner/pull/10

---

### Week 8 (continued) — ANYONECANPAY|NONE Investigation

**Goals:** Research and attempt to implement ANYONECANPAY|NONE sighash
for maximum blockspace efficiency as discussed in the Delving Bitcoin thread.

**What ANYONECANPAY|NONE would do:**
- Each input signs only itself (ANYONECANPAY)
- Signer commits to no outputs (NONE)
- Miners can batch thousands of dust sweeps permissionlessly
- Maximum blockspace efficiency

**Why we stopped:**
Murch flagged on the bitcoindev mailing list that SIGHASH_NONE|ANYONECANPAY
is unsafe — third parties can steal signed inputs as free fee subsidy at
current fee rates.

Reference: https://groups.google.com/g/bitcoindev/c/pr1z3_j8vTc/m/DqMYltO_AAAJ


**Issue #9 closed** with this finding. The correct future implementation
is ANYONECANPAY|ALL tracked in Issue #4.

---

## Challenges & How I Solved Them

### Challenge 1: Regtest conflicts with existing node
Had an existing signet node running with bitcoin.conf containing signet=1.
Adding -regtest flag caused a conflict error.
**Solution:** Separate config file and datadir for regtest only.

### Challenge 2: Dust UTXOs too small to fund their own sweep
Total dust of ~3000 sats cannot cover fees for a transaction with 7 inputs.
**Solution:** Add the largest clean UTXO as a mandatory funder input.
Dust UTXOs ride as additional inputs. Fees deducted from funder amount.

### Challenge 3: Address<NetworkUnchecked> doesn't implement Display
The bitcoincore-rpc crate returns addresses as Address<NetworkUnchecked>
which cannot be printed directly.
**Solution:** Call assume_checked() before converting to string.

### Challenge 4: Testing without external dependencies
classify_utxos takes ListUnspentResultEntry which is hard to construct
in tests without a live node.
**Solution:** Created owned Utxo type and parallel classify_owned_utxos
function that works entirely with our own types.

### Challenge 5: Address linking in sweep transactions
The original batch sweep linked all dust addresses on-chain.
**Solution:** Changed default to per-UTXO sweep. Added --batch as opt-in.

### Challenge 6: ANYONECANPAY|NONE security vulnerability
Attempted to implement NONE|ANYONECANPAY for maximum blockspace efficiency.
**Solution:** Stopped after Murch flagged the security issue on bitcoindev.
Tracked ALL|ANYONECANPAY as the safe future alternative in Issue #4.

---

## What I Learned

**Bitcoin protocol:**
- How dust attacks work and why they are a privacy threat
- Why dust thresholds differ per script type (byte cost of spending)
- How PSBTs enable offline signing workflows (BIP174)
- The PSBT lifecycle: Creator → Updater → Signer → Finalizer → Extractor
- Why SIGHASH_NONE|ANYONECANPAY is unsafe (inputs can be stolen as fee subsidy)
- The difference between ALL|ANYONECANPAY and NONE|ANYONECANPAY
- Why per-UTXO sweeping prevents address linking vs batch sweeping

**Rust:**
- Module system: lib.rs as public interface + separate module files
- Error handling with anyhow and custom error messages
- CLI design with clap derive macros including env var support
- Writing testable code by owning your types
- Lifetime elision rules (clippy caught needless_lifetimes)

**Open source:**
- Importance of reading mailing lists (bitcoindev) alongside GitHub issues
- How to engage with the Bitcoin developer community constructively
- Cross-referencing the BIP draft for alignment
- The value of flagging security issues before shipping them

---

## Community Engagement

- Referenced the Delving Bitcoin discussion throughout implementation
- Received feedback from @haris in BOSS Discord on address linking issue
- Implemented the fix (per-UTXO sweep) based on community feedback
- Stopped ANYONECANPAY|NONE implementation after security finding
- Tracked the ddust BIP PR: https://github.com/bitcoin/bips/pull/2150

---

## Open Issues

- **Issue #4:** feat: implement ANYONECANPAY|ALL sighash for OP_RETURN dust sweep
  - The safe sighash type per Murch's feedback
  - Requires lower-level rust-bitcoin construction
  - Miners can batch permissionlessly without security risk

---

## Future Improvements

- **ANYONECANPAY|ALL sighash** — safe miner-batchable sweep (Issue #4)
- **Staggered broadcast** — random delays between per-UTXO broadcasts
  to prevent timing correlation
- **BIP329 label export** — tag swept UTXOs in Sparrow-compatible format
- **Hardware wallet support** — export PSBTs for Ledger/Coldcard/Trezor
- **Address clustering heuristics** — score dust UTXOs by attack likelihood
- **Watch-only wallet support** — scan without a hot wallet
- **Private broadcast integration** — Bitcoin Core v31 privatebroadcast flag