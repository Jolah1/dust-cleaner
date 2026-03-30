# Design Document: dust-cleaner

**Project:** dust-cleaner  
**Author:** Jolah1     
**Status:** Active development

---

## Problem

Bitcoin privacy can be compromised by dust attacks. An attacker sends sub-threshold amounts of Bitcoin to known addresses. When the victim later spends those UTXOs alongside their real funds, the attacker clusters addresses together to de-anonymize the wallet on-chain.

Most wallets have no tooling to detect or respond to dust attacks. Users are left exposed without knowing it.

**References:**
- [Dust attack explained](https://www.investopedia.com/terms/d/dusting-attack.asp)
- [Disposing of dust attack UTXOs — Delving Bitcoin](https://delvingbitcoin.org/t/disposing-of-dust-attack-utxos/2215/)

---

## Goals

- Detect dust UTXOs in a Bitcoin Core wallet using accurate per-script-type thresholds
- Generate a [BIP174](https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki) PSBT to sweep dust UTXOs in a single transaction
- Provide a dry-run mode to preview the sweep before committing
- Keep the tool simple, auditable, and composable with Bitcoin Core
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
            └── live → PSBT Builder (walletcreatefundedpsbt)
                            │
                            ▼
                        Output (base64 PSBT for signing)
```

---

## Module Responsibilities

### `main.rs`
CLI entry point only. Parses arguments, connects to node, routes to the correct command handler. Contains no business logic.

### `lib.rs`
Declares and re-exports all modules as the public interface. Allows logic to be tested independently of the binary. Follows the pattern used by [rust-bitcoin](https://github.com/rust-bitcoin/rust-bitcoin) and [BDK](https://github.com/bitcoindevkit/bdk).

### `cli.rs`
Defines the CLI interface using [`clap`](https://docs.rs/clap/latest/clap/) derive macros. Responsible for parsing RPC credentials, the `--threshold` flag, subcommands, and the `--dry-run` flag. Credentials are passed as CLI arguments and never stored on disk.

### `rpc.rs`
Establishes the RPC connection to Bitcoin Core using the [`bitcoincore-rpc`](https://docs.rs/bitcoincore-rpc/latest/bitcoincore_rpc/) crate. Connection errors are mapped to human-readable messages with recovery tips.

### `scanner.rs`
Calls [`listunspent`](https://developer.bitcoin.org/reference/rpc/listunspent.html) via RPC and returns the raw UTXO list. Intentionally has no logic — only data fetching. Errors are mapped to friendly messages (node not running, no wallet loaded).

### `analyzer.rs`
Contains all dust detection logic:

- `detect_script_type(address)` — detects P2PKH, P2WPKH, P2TR, P2SH from address prefix
- `is_dust(amount, threshold)` — flat threshold check
- `is_dust_smart(amount, address, user_threshold)` — per-type threshold check with user override
- `classify_utxos_smart(utxos, user_threshold)` — classifies all UTXOs using smart thresholds
- `classify_owned_utxos(utxos, threshold)` — classifies our own `Utxo` type for testing

### `psbt_builder.rs`
Constructs the sweep PSBT and provides dry-run estimation:

- `build_sweep_psbt(client, dust_utxos, clean_utxos)` — creates a real PSBT via [`walletcreatefundedpsbt`](https://developer.bitcoin.org/reference/rpc/walletcreatefundedpsbt.html)
- `dry_run_sweep(dust_utxos, clean_utxos)` — estimates fee and output without creating a PSBT

### `types.rs`
Defines the owned `Utxo` struct used for testing. Decouples test logic from the `bitcoincore-rpc` crate's `ListUnspentResultEntry` type, which is hard to construct without a live node.

---

## Dust Thresholds

The dust threshold is defined as the minimum UTXO value where the fee cost to spend it is less than the UTXO's own value. Since different script types produce inputs of different byte sizes, their fee costs differ.

| Script type | Input size  | Dust threshold | Reference |
|-------------|-------------|----------------|-----------|
| P2PKH       | 148 vbytes  | 546 sats       | [Bitcoin Core source](https://github.com/bitcoin/bitcoin/blob/master/src/policy/policy.cpp) |
| P2WPKH      | 68 vbytes   | 294 sats       | [Bitcoin Core source](https://github.com/bitcoin/bitcoin/blob/master/src/policy/policy.cpp) |
| P2TR        | 58 vbytes   | 294 sats       | [Bitcoin Core source](https://github.com/bitcoin/bitcoin/blob/master/src/policy/policy.cpp) |
| P2SH        | 91 vbytes   | 540 sats       | [Bitcoin Core source](https://github.com/bitcoin/bitcoin/blob/master/src/policy/policy.cpp) |

Script type is detected from address prefix:

| Prefix | Network | Script type |
|--------|---------|-------------|
| `1`    | Mainnet | P2PKH |
| `3`    | Mainnet | P2SH |
| `bc1q` | Mainnet | P2WPKH |
| `bc1p` | Mainnet | P2TR |
| `bcrt1q` | Regtest | P2WPKH |
| `bcrt1p` | Regtest | P2TR |
| `tb1q` | Testnet | P2WPKH |
| `tb1p` | Testnet | P2TR |

---

## PSBT Sweep Strategy

Dust UTXOs alone cannot cover transaction fees. The sweep strategy is:

1. Pick the largest clean UTXO as the fee funder (first input)
2. Add all dust UTXOs as additional mandatory inputs
3. Get a fresh wallet address for the consolidated output
4. Set output amount to the funder's full value
5. Use [`subtractFeeFromOutputs`](https://developer.bitcoin.org/reference/rpc/walletcreatefundedpsbt.html) so Bitcoin Core calculates the exact fee and deducts it automatically
6. Return the base64 PSBT for the user to sign externally

This keeps the tool in the **Creator** role per [BIP174](https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki). Signing is left to the user via `walletprocesspsbt`.

### BIP174 Roles

| Role | Who does it | How |
|------|-------------|-----|
| Creator | dust-cleaner | `walletcreatefundedpsbt` |
| Updater | Bitcoin Core | automatic |
| Signer | User | `walletprocesspsbt` |
| Finalizer | User | `finalizepsbt` |
| Extractor | User | `sendrawtransaction` |

---

## Dry Run Mode

When `--dry-run` is passed, no PSBT is created. The tool estimates the sweep outcome using:

```
estimated_vbytes = (total_inputs × 68) + 31 + 10
estimated_fee    = estimated_vbytes × 2  (2 sat/vbyte conservative)
estimated_output = funder_sats + total_dust_sats - estimated_fee
```

This gives the user a preview before committing to the sweep.

---

## Security Considerations

- Credentials are passed as CLI arguments, never stored on disk or in config files
- The tool never signs transactions — signing is always left to the user
- The tool never broadcasts transactions — broadcasting is always left to the user
- The tool only reads wallet data and creates unsigned PSBTs
- All development and testing is done on regtest before any mainnet use
- No external APIs are called — all data comes from the user's own Bitcoin Core node

---

## Testing

15 unit tests covering:

- `test_is_dust_default_threshold` — flat threshold with default value
- `test_is_dust_custom_threshold` — flat threshold with custom value
- `test_is_dust_zero_threshold` — edge case: zero threshold
- `test_detect_script_type_p2pkh` — address prefix detection
- `test_detect_script_type_p2sh` — address prefix detection
- `test_detect_script_type_p2wpkh` — address prefix detection (mainnet + regtest)
- `test_detect_script_type_p2tr` — address prefix detection
- `test_dust_thresholds_per_type` — correct sat values per type
- `test_is_dust_smart_p2wpkh` — smart threshold at boundary
- `test_is_dust_smart_user_override` — user threshold overrides per-type
- `test_classify_owned_utxos_splits_correctly` — mixed UTXO set
- `test_classify_owned_utxos_all_dust` — all dust edge case
- `test_classify_owned_utxos_all_clean` — all clean edge case
- `test_classify_owned_utxos_empty` — empty UTXO set
- `test_classify_owned_utxos_custom_threshold` — classification with custom threshold

---

## Future Improvements

- **OP_RETURN sweep method** — burn dust to miner fees with no output, more private than consolidating to a new address ([reference](https://delvingbitcoin.org/t/disposing-of-dust-attack-utxos/2215/2))
- **Mempool batching** — check mempool for existing unconfirmed sweep transactions and batch them together to save ~23 vbytes per input
- **Staggered broadcast** — schedule sweeps with random delays to prevent timing correlation
- **BIP329 label export** — tag swept UTXOs as dust-attack in [BIP329](https://github.com/bitcoin/bips/blob/master/bip-0329.mediawiki) format compatible with Sparrow Wallet
- **Hardware wallet support** — export PSBTs for Ledger/Coldcard/Trezor via Sparrow or Specter
- **Address clustering heuristics** — score each dust UTXO by attack likelihood (amount pattern, sender address history, timing)
- **Watch-only wallet support** — scan without needing a hot wallet

---

## References

- [BIP174 — Partially Signed Bitcoin Transactions](https://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki)
- [BIP370 — PSBTv2](https://github.com/bitcoin/bips/blob/master/bip-0370.mediawiki)
- [BIP329 — Wallet Labels](https://github.com/bitcoin/bips/blob/master/bip-0329.mediawiki)
- [Bitcoin Core RPC documentation](https://developer.bitcoin.org/reference/rpc/)
- [Bitcoin Core dust policy source](https://github.com/bitcoin/bitcoin/blob/master/src/policy/policy.cpp)
- [rust-bitcoin crate](https://docs.rs/bitcoin/latest/bitcoin/)
- [bitcoincore-rpc crate](https://docs.rs/bitcoincore-rpc/latest/bitcoincore_rpc/)
- [clap — CLI framework for Rust](https://docs.rs/clap/latest/clap/)
- [anyhow — error handling for Rust](https://docs.rs/anyhow/latest/anyhow/)
- [Disposing of dust attack UTXOs — Delving Bitcoin](https://delvingbitcoin.org/t/disposing-of-dust-attack-utxos/2215/)
- [Original project idea — 0xB10C](https://github.com/0xB10C/project-ideas/issues/13)
- [ddust — reference implementation](https://github.com/bubb1es71/ddust)