# Design Document: dust-cleaner

**Project:** dust-cleaner
**Author:** Jolah1
**Program:** [BOSS 2026](https://learning.chaincode.com/)
**Based on:** [0xB10C project idea #13](https://github.com/0xB10C/project-ideas/issues/13)
**Version:** 0.1.0
**Status:** Active development

---

## Problem

Bitcoin privacy can be compromised by dust attacks. An attacker sends
sub-threshold amounts of Bitcoin to known addresses. When the victim later
spends those UTXOs alongside their real funds, the attacker clusters addresses
together to de-anonymize the wallet on-chain.

Most wallets have no tooling to detect or respond to dust attacks. Users are
left exposed without knowing it.

**Key insight from building this tool:**
Even a tool that sweeps dust can worsen privacy if it batches UTXOs from
multiple addresses into a single transaction — that's exactly what the attacker
wants. The sweep strategy is as important as the detection.

**References:**
- [Dust attack explained](https://www.investopedia.com/terms/d/dusting-attack.asp)
- [Disposing of dust attack UTXOs — Delving Bitcoin](https://delvingbitcoin.org/t/disposing-of-dust-attack-utxos/2215/)
- [Dust UTXO Disposal Protocol — BIP draft](https://github.com/bitcoin/bips/pull/2150)

---

## Goals

- Detect dust UTXOs in a Bitcoin Core wallet using accurate per-script-type thresholds
- Generate [BIP174](https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki) PSBTs to sweep dust UTXOs without linking addresses
- Default to the most private behavior — sweep each UTXO separately
- Provide dry-run mode to preview the sweep before committing
- Never touch keys or broadcast transactions automatically

---

## Non-Goals

- This tool does not broadcast transactions automatically
- This tool does not manage keys or sign transactions directly
- This tool does not support hardware wallets (yet)
- This tool does not implement address clustering heuristics (yet)

---

## Architecture

```
CLI (clap)
    │
    ├── scan command
    │       │
    │       ▼
    │   RPC Connection (bitcoincore-rpc)
    │       │
    │       ▼
    │   UTXO Scanner (list_unspent)
    │       │
    │       ▼
    │   Dust Analyzer (per-script-type classification)
    │       │
    │       ▼
    │   Output (formatted UTXO list + summary)
    │
    └── sweep command
            │
            ├── --dry-run → Fee estimator (no PSBT created)
            │
            ├── default (per-UTXO) → One PSBT per dust UTXO
            │                         No address linking
            │
            └── --batch → Single PSBT for all dust UTXOs
                           Faster but links addresses
```

---

## Module Responsibilities

### `main.rs`
CLI entry point only. Parses arguments, connects to node, calls
`handle_scan` or `handle_sweep`. Contains no business logic.

### `lib.rs`
Declares and re-exports all modules as the public interface. Allows logic
to be tested independently of the binary. Follows the pattern used by
[rust-bitcoin](https://github.com/rust-bitcoin/rust-bitcoin) and
[BDK](https://github.com/bitcoindevkit/bdk).

### `cli.rs`
Defines the CLI interface using [`clap`](https://docs.rs/clap/latest/clap/)
derive macros. All flags support environment variables via clap's `env`
attribute so users can set credentials once per session.

**Flags:**
- `--rpc-url` / `DUST_RPC_URL`
- `--rpc-user` / `DUST_RPC_USER`
- `--rpc-pass` / `DUST_RPC_PASS`
- `--threshold` / `DUST_THRESHOLD`
- `--method` (consolidate | op-return)
- `--batch` (opt-in batching)
- `--dry-run`

### `rpc.rs`
Establishes the RPC connection to Bitcoin Core using
[`bitcoincore-rpc`](https://docs.rs/bitcoincore-rpc/latest/bitcoincore_rpc/).
Connection errors are mapped to human-readable messages with recovery tips.

### `scanner.rs`
Calls [`listunspent`](https://developer.bitcoin.org/reference/rpc/listunspent.html)
via RPC and returns the raw UTXO list. Intentionally has no logic — only
data fetching. Errors are mapped to friendly messages.

### `analyzer.rs`
Contains all dust detection logic:

- `detect_script_type(address)` — detects P2PKH, P2WPKH, P2TR, P2SH from address prefix
- `is_dust(amount, threshold)` — flat threshold check
- `is_dust_smart(amount, address, user_threshold)` — per-type threshold with user override
- `classify_utxos_smart(utxos, user_threshold)` — classifies all UTXOs
- `classify_owned_utxos(utxos, threshold)` — classifies our own Utxo type for testing

### `psbt_builder.rs`
Constructs sweep PSBTs with shared private helper functions:

- `select_funder(clean_utxos)` — picks largest clean UTXO to fund fees
- `build_inputs(funder, dust_utxos)` — constructs input list
- `build_sweep_psbt(...)` — consolidate method, batch mode
- `build_op_return_psbt(...)` — OP_RETURN method, batch mode
- `build_per_utxo_psbts(...)` — per-UTXO mode, one PSBT each
- `dry_run_sweep(...)` — estimates fee and output without creating a PSBT

### `types.rs`
Defines the owned `Utxo` struct used for testing. Decouples test logic
from the `bitcoincore-rpc` crate's `ListUnspentResultEntry` type.

---

## Dust Thresholds

The dust threshold is defined as the minimum UTXO value where the fee cost
to spend it is less than the UTXO's own value. Different script types produce
inputs of different byte sizes, so their fee costs differ.

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
| `bc1q` | Mainnet | P2WPKH |
| `bc1p` | Mainnet | P2TR |
| `bcrt1q` | Regtest | P2WPKH |
| `bcrt1p` | Regtest | P2TR |
| `tb1q` | Testnet | P2WPKH |
| `tb1p` | Testnet | P2TR |

---

## Sweep Strategy

### The Core Problem

A naive sweep that batches all dust UTXOs in one transaction is a privacy
failure — it links all dust addresses on-chain, completing the attacker's goal.

```
WRONG (batch):
Input 1: dust from address A ─┐
Input 2: dust from address B ─┼─→ output  ← addresses now linked
Input 3: dust from address C ─┘
```

### Default: Per-UTXO Sweep (most private)

Each dust UTXO is swept in its own transaction. No on-chain link between
addresses from different UTXOs.

```
Tx 1: dust from address A → OP_RETURN  (no link to B or C)
Tx 2: dust from address B → OP_RETURN  (no link to A or C)
Tx 3: dust from address C → OP_RETURN  (no link to A or B)
```

Each transaction:
1. Takes the dust UTXO as input
2. Adds the largest clean UTXO to fund fees
3. Produces an OP_RETURN "ash" output (0 sats)
4. Fees deducted from change back to wallet

### Opt-in: Batch Sweep (--batch flag)

For users who prioritize UTXO consolidation over privacy. All dust UTXOs
in one transaction — faster but links addresses.

### Sweep Methods

| Method | Output | Address linking | Use case |
|--------|--------|----------------|----------|
| op-return (default) | OP_RETURN "ash" | Only if --batch | Maximum privacy |
| consolidate | Fresh wallet address | Only if --batch | UTXO set management |

### BIP174 Roles

This tool acts as the **Creator** role per BIP174. Signing is left to the user.

| Role | Who | How |
|------|-----|-----|
| Creator | dust-cleaner | `walletcreatefundedpsbt` |
| Updater | Bitcoin Core | automatic |
| Signer | User | `walletprocesspsbt` |
| Finalizer | User | `finalizepsbt` |
| Extractor | User | `sendrawtransaction` |

---

## Sighash Research: ANYONECANPAY

### What was attempted

We investigated using `SIGHASH_NONE|ANYONECANPAY` for maximum blockspace
efficiency. Under this scheme:
- Each input signs only itself (ANYONECANPAY)
- Signer commits to no outputs (NONE)
- Miners can batch thousands of dust sweeps permissionlessly

This was discussed in the Delving Bitcoin thread and implemented by the
ddust team before being reverted.

### Why NONE|ANYONECANPAY is unsafe

Murch flagged on the bitcoindev mailing list that `SIGHASH_NONE|ANYONECANPAY`
lets third parties steal signed inputs as free fee subsidy at current fee rates.
Since the signer commits to no outputs, anyone can take the signed input and
use it for their own transaction.

Reference: https://groups.google.com/g/bitcoindev/c/pr1z3_j8vTc/m/DqMYltO_AAAJ

The ddust team reverted their implementation:
https://github.com/bubb1es71/ddust/pull/28

### The safe alternative: ALL|ANYONECANPAY

`SIGHASH_ALL|ANYONECANPAY` allows miners to add more inputs (batching)
while the signer commits to all outputs. This prevents the fee-stealing
attack while still enabling permissionless batching.

This is tracked in Issue #4 for future implementation. It requires
lower-level transaction construction using `rust-bitcoin` directly since
`walletcreatefundedpsbt` does not support custom sighash types.

---

## Dry Run Mode

When `--dry-run` is passed, no PSBT is created. The tool estimates:

```
estimated_vbytes = (total_inputs × 68) + 31 + 10
estimated_fee    = estimated_vbytes × 2  (2 sat/vbyte conservative)
estimated_output = funder_sats + total_dust_sats - estimated_fee
```

---

## Security Considerations

- Credentials are passed as CLI flags or env vars, never stored on disk
- The tool never signs transactions — signing is always left to the user
- The tool never broadcasts transactions — broadcasting is always left to the user
- The tool only reads wallet data and creates unsigned PSBTs
- Default behavior (per-UTXO) prevents address linking
- ANYONECANPAY|NONE was rejected after security review (Murch finding)
- All development and testing done on regtest before mainnet use

---

## Testing

15 unit tests covering:

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

---

## Future Improvements

- **ANYONECANPAY|ALL sighash** (Issue #4) — safe miner-batchable sweep
  using rust-bitcoin direct construction
- **Staggered broadcast** — random delays between per-UTXO broadcasts
  to prevent timing correlation between addresses
- **BIP329 label export** — tag swept UTXOs in Sparrow-compatible format
- **Hardware wallet support** — export PSBTs for Ledger/Coldcard/Trezor
- **Address clustering heuristics** — score each UTXO by attack likelihood
- **Watch-only wallet support** — scan without a hot wallet
- **Private broadcast** — Bitcoin Core v31 privatebroadcast flag integration

---

## References

- [BIP174 — Partially Signed Bitcoin Transactions](https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki)
- [BIP370 — PSBTv2](https://github.com/bitcoin/bips/blob/master/bip-0370.mediawiki)
- [BIP329 — Wallet Labels](https://github.com/bitcoin/bips/blob/master/bip-0329.mediawiki)
- [BIP143 — Sighash types](https://github.com/bitcoin/bips/blob/master/bip-0143.mediawiki)
- [Dust UTXO Disposal Protocol — BIP draft](https://github.com/bitcoin/bips/pull/2150)
- [Bitcoin Core RPC documentation](https://developer.bitcoin.org/reference/rpc/)
- [Bitcoin Core dust policy](https://github.com/bitcoin/bitcoin/blob/master/src/policy/policy.cpp)
- [ANYONECANPAY|NONE security finding — bitcoindev](https://groups.google.com/g/bitcoindev/c/pr1z3_j8vTc/m/DqMYltO_AAAJ)
- [rust-bitcoin crate](https://docs.rs/bitcoin/latest/bitcoin/)
- [bitcoincore-rpc crate](https://docs.rs/bitcoincore-rpc/latest/bitcoincore_rpc/)