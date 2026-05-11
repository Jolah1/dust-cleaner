# Design Document: dust-cleaner

**Project:** dust-cleaner
**Author:** Jolah1
---

## Problem

Bitcoin privacy can be compromised by dust attacks. An attacker sends
sub-threshold amounts of Bitcoin to known addresses. When the victim later
spends those UTXOs alongside their real funds, the attacker clusters addresses
together to de-anonymize the wallet on-chain.

Most wallets have no tooling to detect or respond to dust attacks.

**Critical insight:** Even a sweep tool can worsen privacy if it batches UTXOs
from multiple addresses into one transaction. The sweep strategy is as important
as the detection.

**References:**
- [Dust attack explained](https://www.investopedia.com/terms/d/dusting-attack.asp)
- [Disposing of dust attack UTXOs — Delving Bitcoin](https://delvingbitcoin.org/t/disposing-of-dust-attack-utxos/2215/)
- [Dust UTXO Disposal Protocol — BIP draft](https://github.com/bitcoin/bips/pull/2150)

---

## Goals

- Detect dust UTXOs using accurate per-script-type thresholds
- Default to the most private sweep behavior — one UTXO per transaction
- Support ANYONECANPAY|ALL sighash for miner-batchable sweeps
- Never touch keys or broadcast transactions automatically
- Provide dry-run mode to preview sweeps before committing

---

## Non-Goals

- Does not broadcast transactions automatically
- Does not manage or store private keys
- Does not support hardware wallets (yet)
- Does not implement address clustering heuristics (yet)

---

## Architecture

```
CLI (clap + env vars)
    │
    ├── scan
    │     │
    │     ▼
    │   RPC → listunspent → Dust Analyzer → Output
    │
    └── sweep
          │
          ├── --dry-run → Fee estimator (no tx created)
          │
          ├── --method anyone-can-pay
          │     │
          │     ▼
          │   getrawtransaction → createrawtransaction
          │     → signrawtransactionwithwallet (ALL|ANYONECANPAY)
          │     → raw signed hex (no funder needed)
          │
          ├── default (per-UTXO)
          │     │
          │     ▼
          │   One walletcreatefundedpsbt per dust UTXO
          │     → base64 PSBT per UTXO
          │
          └── --batch
                │
                ▼
              Single walletcreatefundedpsbt for all dust UTXOs
                → one base64 PSBT (links addresses)
```

---

## Module Responsibilities

### `main.rs`
CLI entry point only. Routes to `handle_scan` or `handle_sweep`. No logic.

### `lib.rs`
Declares and re-exports all modules as public interface. Enables testing
logic independently of the binary.

### `cli.rs`
Defines CLI interface using clap derive macros. All flags support env vars.

**Flags:**
- `--rpc-url` / `DUST_RPC_URL`
- `--rpc-user` / `DUST_RPC_USER`
- `--rpc-pass` / `DUST_RPC_PASS`
- `--threshold` / `DUST_THRESHOLD`
- `--method` (consolidate | op-return | anyone-can-pay)
- `--batch` (opt-in batching)
- `--dry-run`

### `rpc.rs`
RPC connection to Bitcoin Core. Errors mapped to friendly messages.

### `scanner.rs`
Calls `listunspent` via RPC. No logic — only data fetching.
Errors mapped to helpful recovery tips.

### `analyzer.rs`
All dust detection logic:
- `detect_script_type(address)` — P2PKH, P2WPKH, P2TR, P2SH from prefix
- `is_dust(amount, threshold)` — flat threshold check
- `is_dust_smart(amount, address, user_threshold)` — per-type with override
- `classify_utxos_smart(utxos, user_threshold)` — classifies all UTXOs
- `classify_owned_utxos(utxos, threshold)` — for testing with owned types

### `psbt_builder.rs`
All sweep logic. Private helpers shared across methods:
- `select_funder(clean_utxos)` — picks largest clean UTXO for fees
- `build_inputs(funder, dust_utxos)` — constructs input list

Public functions:
- `build_sweep_psbt(...)` — consolidate, batch mode
- `build_op_return_psbt(...)` — OP_RETURN, batch mode
- `build_per_utxo_psbts(...)` — per-UTXO mode, one PSBT each
- `build_anyonecanpay_all_txs(...)` — ANYONECANPAY|ALL, returns raw signed hex
- `dry_run_sweep(...)` — estimates fee and output, no tx created

### `types.rs`
Owned `Utxo` struct for testing. Decouples tests from bitcoincore-rpc types.

---

## Dust Thresholds

| Script type | Input size  | Dust threshold | Source |
|-------------|-------------|----------------|--------|
| P2PKH       | 148 vbytes  | 546 sats       | [Bitcoin Core policy](https://github.com/bitcoin/bitcoin/blob/master/src/policy/policy.cpp) |
| P2WPKH      | 68 vbytes   | 294 sats       | [Bitcoin Core policy](https://github.com/bitcoin/bitcoin/blob/master/src/policy/policy.cpp) |
| P2TR        | 58 vbytes   | 294 sats       | [Bitcoin Core policy](https://github.com/bitcoin/bitcoin/blob/master/src/policy/policy.cpp) |
| P2SH        | 91 vbytes   | 540 sats       | [Bitcoin Core policy](https://github.com/bitcoin/bitcoin/blob/master/src/policy/policy.cpp) |

Script type detected from address prefix:

| Prefix | Network | Script type |
|--------|---------|-------------|
| `1` | Mainnet | P2PKH |
| `3` | Mainnet | P2SH |
| `bc1q` / `bcrt1q` / `tb1q` | Any | P2WPKH |
| `bc1p` / `bcrt1p` / `tb1p` | Any | P2TR |

---

## Sweep Strategies

### The Privacy Problem with Batching

Batching all dust UTXOs into one transaction links all their addresses:

```
Input 1: dust from address A ─┐
Input 2: dust from address B ─┼─→ output  ← A, B, C now linked on-chain
Input 3: dust from address C ─┘
```

This is exactly what the attacker wants.

### Default: Per-UTXO (most private)

One transaction per dust UTXO. No on-chain address linking.

```
Tx 1: dust A → OP_RETURN  (isolated)
Tx 2: dust B → OP_RETURN  (isolated)
Tx 3: dust C → OP_RETURN  (isolated)
```

Uses `walletcreatefundedpsbt`. Requires a clean UTXO to fund fees.
Returns base64 PSBTs for user to sign and broadcast separately.

### ANYONECANPAY|ALL (most private + miner batchable)

Each dust UTXO signed independently with `SIGHASH_ALL|ANYONECANPAY`:
- **ANYONECANPAY** — input signs only itself; miners can add inputs
- **ALL** — all outputs committed; miners cannot change OP_RETURN output

No funder UTXO needed — the dust value itself becomes the fee.
Returns raw signed hex ready for broadcast.

```
Tx: dust A input → OP_RETURN "ash" (0 sats)
    signed with ALL|ANYONECANPAY
    miners can add inputs but cannot change outputs
```

**Implementation:**
```
getrawtransaction (verbose) → get scriptPubKey
createrawtransaction → build tx with OP_RETURN "ash" output
signrawtransactionwithwallet → sign with "ALL|ANYONECANPAY"
→ raw signed hex
```

Requires `txindex=1` in `bitcoin.conf`.

**Why not NONE|ANYONECANPAY:**
Murch flagged that NONE|ANYONECANPAY is unsafe — third parties can steal
signed inputs as fee subsidy since no outputs are committed.
Reference: https://groups.google.com/g/bitcoindev/c/pr1z3_j8vTc/m/DqMYltO_AAAJ

### Opt-in: Batch (--batch flag)

All dust UTXOs in one transaction. Faster but links addresses.
User must explicitly opt in. Privacy warning shown automatically.

---

## BIP174 Roles

| Role | Who | How |
|------|-----|-----|
| Creator | dust-cleaner | `walletcreatefundedpsbt` or `createrawtransaction` |
| Updater | Bitcoin Core | automatic |
| Signer | User (or wallet) | `walletprocesspsbt` or `signrawtransactionwithwallet` |
| Finalizer | User | `finalizepsbt` |
| Extractor | User | `sendrawtransaction` |

For ANYONECANPAY|ALL, signing happens inside dust-cleaner via
`signrawtransactionwithwallet`. The output is already a signed raw tx,
not a PSBT.

---

## Dry Run Mode

Estimates fee without creating a transaction:

```
estimated_vbytes = (total_inputs × 68) + 31 + 10
estimated_fee    = estimated_vbytes × 2  (2 sat/vbyte conservative)
estimated_output = funder_sats + total_dust_sats - estimated_fee
```

For ANYONECANPAY|ALL, dry-run shows total dust going to fees with no
funder needed.

---

## Security Considerations

- Credentials via CLI flags or env vars, never stored on disk
- Tool never auto-broadcasts — user controls broadcasting
- Default sweep prevents address linking (per-UTXO)
- ANYONECANPAY|NONE rejected after Murch's security finding
- ALL|ANYONECANPAY locks outputs — miners can add inputs but not steal
- All development and testing on regtest before mainnet
- `txindex=1` required for ANYONECANPAY|ALL method

---

## Testing

15 unit tests — all passing:
- `test_is_dust_default_threshold`
- `test_is_dust_custom_threshold`
- `test_is_dust_zero_threshold`
- `test_detect_script_type_p2pkh`
- `test_detect_script_type_p2sh`
- `test_detect_script_type_p2wpkh`
- `test_detect_script_type_p2tr`
- `test_dust_thresholds_per_type`
- `test_is_dust_smart_p2wpkh`
- `test_is_dust_smart_user_override`
- `test_classify_owned_utxos_splits_correctly`
- `test_classify_owned_utxos_all_dust`
- `test_classify_owned_utxos_all_clean`
- `test_classify_owned_utxos_empty`
- `test_classify_owned_utxos_custom_threshold`

CI runs on every push: test + clippy + fmt.

---

## Future Improvements

- **Staggered broadcast** — random delays between per-UTXO broadcasts
- **BIP329 label export** — Sparrow-compatible dust-attack labels
- **Hardware wallet support** — Ledger/Coldcard/Trezor via Sparrow
- **Address clustering heuristics** — score UTXOs by attack likelihood
- **Watch-only wallet support**
- **Private broadcast** — Bitcoin Core v31 privatebroadcast flag
- **Automatic broadcast pipeline** — pipe hex directly to sendrawtransaction

---

## References

- [BIP174 — PSBTs](https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki)
- [BIP143 — Sighash](https://github.com/bitcoin/bips/blob/master/bip-0143.mediawiki)
- [Dust UTXO Disposal Protocol — BIP draft](https://github.com/bitcoin/bips/pull/2150)
- [Bitcoin Core RPC docs](https://developer.bitcoin.org/reference/rpc/)
- [Bitcoin Core dust policy](https://github.com/bitcoin/bitcoin/blob/master/src/policy/policy.cpp)
- [ANYONECANPAY|NONE security finding](https://groups.google.com/g/bitcoindev/c/pr1z3_j8vTc/m/DqMYltO_AAAJ)
- [rust-bitcoin](https://docs.rs/bitcoin/latest/bitcoin/)
- [bitcoincore-rpc](https://docs.rs/bitcoincore-rpc/latest/bitcoincore_rpc/)
- [Delving Bitcoin thread](https://delvingbitcoin.org/t/disposing-of-dust-attack-utxos/2215/)