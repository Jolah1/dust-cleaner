# Design Document: dust-cleaner

## Problem

Bitcoin privacy can be compromised by dust attacks. An attacker sends 
sub-threshold amounts of Bitcoin to known addresses. When the victim 
spends those UTXOs, the attacker clusters addresses together to 
de-anonymize the wallet.

Most wallets have no tooling to detect or respond to dust attacks. 
Users are left exposed without knowing it.

## Goals

- Detect dust UTXOs in a Bitcoin Core wallet
- Generate a PSBT to sweep dust UTXOs in a single transaction
- Keep the tool simple, auditable, and composable with Bitcoin Core

## Non-Goals

- This tool does not broadcast transactions automatically
- This tool does not manage keys or sign transactions directly
- This tool does not support hardware wallets (yet)

## Architecture
```
CLI (clap)
    │
    ▼
RPC Connection (bitcoincore-rpc)
    │
    ▼
UTXO Scanner (list_unspent)
    │
    ▼
Dust Analyzer (threshold classification)
    │
    ▼
PSBT Builder (walletcreatefundedpsbt)
    │
    ▼
Output (base64 PSBT for signing)
```

## Module Responsibilities

### `cli.rs`
Defines the CLI interface using `clap`. Responsible for parsing 
user-provided arguments including RPC credentials and subcommands.

### `rpc.rs`
Establishes the RPC connection to Bitcoin Core. Accepts URL, 
username, and password as parameters so credentials never touch 
the filesystem.

### `scanner.rs`
Calls `list_unspent` via RPC and returns the raw UTXO list. 
Intentionally has no logic — only data fetching.

### `analyzer.rs`
Contains all dust detection logic. The `is_dust` function checks 
whether a UTXO's value is below the dust threshold. `classify_utxos` 
splits the UTXO list into dust and clean buckets.

**Dust threshold:** 1000 satoshis (conservative, covers P2PKH, 
P2WPKH, and P2TR output types).

### `psbt_builder.rs`
Constructs the sweep PSBT. Takes dust UTXOs as mandatory inputs 
and adds one clean UTXO to fund transaction fees. Uses 
`walletcreatefundedpsbt` with `subtractFeeFromOutputs` so fees 
are deducted automatically from the output amount.

## PSBT Sweep Strategy

Dust UTXOs alone cannot cover transaction fees. The sweep 
strategy is:

1. Select all dust UTXOs as mandatory inputs
2. Add the largest clean UTXO as a fee funder
3. Set a single output to a fresh wallet address
4. Use `subtractFeeFromOutputs` so Bitcoin Core calculates 
   the exact fee and deducts it from the output
5. Return the base64 PSBT for the user to sign externally

This keeps the tool in the Creator + Updater role per BIP174. 
Signing is left to the user via `walletprocesspsbt`.

## Security Considerations

- Credentials are passed as CLI arguments, never stored on disk
- The tool never signs or broadcasts transactions
- The tool only reads wallet data and creates unsigned PSBTs
- All testing is done on regtest before mainnet use

## Future Improvements

- `--threshold` flag to let users set a custom dust threshold
- Per-script-type thresholds (P2PKH vs P2WPKH vs P2TR)
- Dry-run mode to preview sweep without creating a PSBT
- Address clustering heuristics to score dust attack likelihood
- Support for watch-only wallets