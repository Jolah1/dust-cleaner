mod analyzer;
mod cli;
mod psbt_builder;
mod rpc;
mod scanner;
mod types;

use bitcoincore_rpc::Client;
use clap::Parser;
use cli::{Cli, Commands, SweepMethod};

fn handle_scan(client: &Client, user_threshold: Option<u64>) -> anyhow::Result<()> {
    let utxos = scanner::fetch_utxos(client)?;
    let total_utxos = utxos.len();

    let threshold_display = match user_threshold {
        Some(t) => format!("{} sats (custom)", t),
        None => "per-script-type (P2PKH:546, P2WPKH:294, P2TR:294, P2SH:540)".to_string(),
    };
    println!(
        "Found {} total UTXOs (threshold: {})\n",
        total_utxos, threshold_display
    );

    let (dust_utxos, clean_utxos) = analyzer::classify_utxos_smart(utxos, user_threshold);

    println!("⚠️  DUST UTXOs ({} found):", dust_utxos.len());
    if dust_utxos.is_empty() {
        println!("   none");
    }
    for utxo in &dust_utxos {
        let addr = utxo
            .address
            .as_ref()
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
    println!(
        "   Dust UTXOs:     {} ({} sats)",
        dust_utxos.len(),
        total_dust_sats
    );
    println!(
        "   Clean UTXOs:    {} ({} sats)",
        clean_utxos.len(),
        total_clean_sats
    );
    println!("   Threshold:      {}", threshold_display);

    if !dust_utxos.is_empty() {
        println!("\n   💡 Run 'sweep' to consolidate dust into a single UTXO");
    } else {
        println!("\n   ✅ Wallet is clean — no dust detected");
    }
    println!("─────────────────────────────────────────");

    Ok(())
}

fn handle_sweep(
    client: &Client,
    user_threshold: Option<u64>,
    dry_run: bool,
    method: SweepMethod,
    batch: bool,
) -> anyhow::Result<()> {
    let utxos = scanner::fetch_utxos(client)?;
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

    if !batch {
        println!("\n🔒 Mode: per-UTXO (default) — each dust UTXO swept separately");
        println!("   No address linking. Use --batch to sweep all at once.\n");
    } else {
        println!("\n⚠️  Mode: batch — all dust UTXOs swept in one transaction");
        println!("   Warning: this links all dust addresses on-chain.\n");
    }

    if dry_run {
        let result = psbt_builder::dry_run_sweep(&dust_utxos, &clean_utxos)?;
        println!("🔍 Dry Run — no PSBT created\n");
        println!("   Method:            {:?}", method);
        println!(
            "   Mode:              {}",
            if batch { "batch" } else { "per-UTXO" }
        );
        println!("   Dust inputs:       {}", result.dust_input_count);
        println!("   Total dust:        {} sats", result.total_dust_sats);
        println!("   Funder UTXO:       {} sats", result.funder_sats);
        println!("   Estimated fee:     {} sats", result.estimated_fee_sats);
        println!(
            "   Estimated output:  {} sats",
            result.estimated_output_sats
        );
        println!("\n   Run without --dry-run to create the PSBT.");
        return Ok(());
    }

    if batch {
        // existing batch behavior
        let result = match method {
            SweepMethod::Consolidate => {
                println!("📎 Method: consolidate — dust swept to fresh address");
                psbt_builder::build_sweep_psbt(client, &dust_utxos, &clean_utxos)?
            }
            SweepMethod::OpReturn => {
                println!("🔥 Method: op-return — dust burned to miner fees");
                psbt_builder::build_op_return_psbt(client, &dust_utxos, &clean_utxos)?
            }
        };

        println!("\n📊 Sweep Summary:");
        println!("   Dust inputs:  {}", result.dust_input_count);
        println!("   Total dust:   {} sats", result.total_dust_sats);
        println!("\n🧹 Sweep PSBT (base64):");
        println!("{}", result.psbt);
        println!("\n💡 Next steps:");
        println!("   Inspect: bitcoin-cli decodepsbt <psbt>");
        println!("   Sign:    bitcoin-cli walletprocesspsbt <psbt>");
        println!("   Send:    bitcoin-cli sendrawtransaction <hex>");
    } else {
        // per-UTXO behavior — one PSBT per dust UTXO
        let results = psbt_builder::build_per_utxo_psbts(client, &dust_utxos, &clean_utxos)?;

        println!(
            "📊 Generated {} PSBTs (one per dust UTXO):\n",
            results.len()
        );

        for (i, (address, result)) in results.iter().enumerate() {
            println!("─── PSBT {} of {} ───", i + 1, results.len());
            println!("   Address:    {}", address);
            println!("   Dust:       {} sats", result.total_dust_sats);
            println!("   PSBT:       {}", result.psbt);
            println!();
        }

        println!("💡 Sign and broadcast each PSBT separately:");
        println!("   Sign:    bitcoin-cli walletprocesspsbt <psbt>");
        println!("   Send:    bitcoin-cli sendrawtransaction <hex>");
        println!("\n⚠️  Broadcast each transaction at different times");
        println!("   to prevent timing correlation between addresses.");
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let client = rpc::connect(&cli.rpc_url, &cli.rpc_user, &cli.rpc_pass)?;
    let user_threshold = cli.threshold;

    match cli.command {
        Commands::Scan => handle_scan(&client, user_threshold)?,
        Commands::Sweep {
            dry_run,
            method,
            batch,
        } => handle_sweep(&client, user_threshold, dry_run, method, batch)?,
    }

    Ok(())
}
