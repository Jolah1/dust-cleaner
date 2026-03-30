mod cli;
mod rpc;
mod scanner;
mod analyzer;
mod psbt_builder;
mod types;

use clap::Parser;
use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let client = rpc::connect(&cli.rpc_url, &cli.rpc_user, &cli.rpc_pass)?;
    let user_threshold = cli.threshold;

    match cli.command {
        Commands::Scan => {
            let utxos = scanner::fetch_utxos(&client)?;
            let total_utxos = utxos.len();

            let threshold_display = match user_threshold {
                Some(t) => format!("{} sats (custom)", t),
                None => "per-script-type (P2PKH:546, P2WPKH:294, P2TR:294, P2SH:540)".to_string(),
            };
            println!("Found {} total UTXOs (threshold: {})\n", total_utxos, threshold_display);

            let (dust_utxos, clean_utxos) = analyzer::classify_utxos_smart(utxos, user_threshold);

            println!("⚠️  DUST UTXOs ({} found):", dust_utxos.len());
            if dust_utxos.is_empty() {
                println!("   none");
            }
            for utxo in &dust_utxos {
                let addr = utxo.address.as_ref()
    .map(|a| a.clone().assume_checked().to_string())
    .unwrap_or_else(|| "unknown".to_string());
                let script_type = analyzer::detect_script_type(&addr);
                println!(
                    "   {} sats | {}:{} | {:?} | {}",
                    utxo.amount.to_sat(),
                    utxo.txid,
                    utxo.vout,
                    script_type,
                    addr
                );
            }

            println!("\n✅ CLEAN UTXOs ({} found):", clean_utxos.len());
            if clean_utxos.is_empty() {
                println!("   none");
            }
            for utxo in &clean_utxos {
                println!(
                    "   {} sats | {}:{}",
                    utxo.amount.to_sat(),
                    utxo.txid,
                    utxo.vout
                );
            }

            let total_dust_sats: u64 = dust_utxos.iter().map(|u| u.amount.to_sat()).sum();
            let total_clean_sats: u64 = clean_utxos.iter().map(|u| u.amount.to_sat()).sum();

            println!("\n─────────────────────────────────────────");
            println!("📊 Summary");
            println!("   Total UTXOs:    {}", total_utxos);
            println!("   Dust UTXOs:     {} ({} sats)", dust_utxos.len(), total_dust_sats);
            println!("   Clean UTXOs:    {} ({} sats)", clean_utxos.len(), total_clean_sats);
            println!("   Threshold:      {}", threshold_display);

            if !dust_utxos.is_empty() {
                println!("\n   💡 Run 'sweep' to consolidate dust into a single UTXO");
            } else {
                println!("\n   ✅ Wallet is clean — no dust detected");
            }
            println!("─────────────────────────────────────────");
        }

        Commands::Sweep { dry_run } => {
            let utxos = scanner::fetch_utxos(&client)?;
            let (dust_utxos, clean_utxos) = analyzer::classify_utxos_smart(utxos, user_threshold);

            if dust_utxos.is_empty() {
                println!("✅ No dust UTXOs found. Wallet is clean!");
                return Ok(());
            }

            println!("Found {} dust UTXOs to sweep:", dust_utxos.len());
            for utxo in &dust_utxos {
                println!(
                    "   {} sats | {}:{}",
                    utxo.amount.to_sat(),
                    utxo.txid,
                    utxo.vout
                );
            }

            if dry_run {
                let result = psbt_builder::dry_run_sweep(&dust_utxos, &clean_utxos)?;
                println!("\n🔍 Dry Run — no PSBT created\n");
                println!("   Dust inputs:       {}", result.dust_input_count);
                println!("   Total dust:        {} sats", result.total_dust_sats);
                println!("   Funder UTXO:       {} sats", result.funder_sats);
                println!("   Estimated fee:     {} sats", result.estimated_fee_sats);
                println!("   Estimated output:  {} sats", result.estimated_output_sats);
                println!("\n   Run without --dry-run to create the PSBT.");
                return Ok(());
            }

            let result = psbt_builder::build_sweep_psbt(&client, &dust_utxos, &clean_utxos)?;
            println!("\n📊 Sweep Summary:");
            println!("   Dust inputs:  {}", result.dust_input_count);
            println!("   Total dust:   {} sats", result.total_dust_sats);
            println!("\n🧹 Sweep PSBT (base64):");
            println!("{}", result.psbt);
            println!("\n💡 Next steps:");
            println!("   Inspect: bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass decodepsbt <psbt>");
            println!("   Sign:    bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass walletprocesspsbt <psbt>");
            println!("   Send:    bitcoin-cli -rpcport=18443 -rpcuser=user -rpcpassword=pass sendrawtransaction <hex>");
        }
    }

    Ok(())
}