use clap::Parser;
use dust_cleaner::cli::{Cli, Commands};
use dust_cleaner::{analyzer, psbt_builder, rpc, scanner};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let client = rpc::connect(&cli.rpc_url, &cli.rpc_user, &cli.rpc_pass)?;

    match cli.command {
        Commands::Scan => {
            let utxos = scanner::fetch_utxos(&client)?;
            println!("Found {} total UTXOs\n", utxos.len());

            let (dust_utxos, clean_utxos) = analyzer::classify_utxos(utxos);

            println!("⚠️  DUST UTXOs ({} found):", dust_utxos.len());
            for utxo in &dust_utxos {
                println!(
                    "   {} sats | {}:{} | address: {:?}",
                    utxo.amount.to_sat(),
                    utxo.txid,
                    utxo.vout,
                    utxo.address
                );
            }

            println!("\n✅ CLEAN UTXOs ({} found):", clean_utxos.len());
            for utxo in &clean_utxos {
                println!(
                    "   {} sats | {}:{}",
                    utxo.amount.to_sat(),
                    utxo.txid,
                    utxo.vout
                );
            }
        }
        Commands::Sweep => {
            let utxos = scanner::fetch_utxos(&client)?;
            let (dust_utxos, clean_utxos) = analyzer::classify_utxos(utxos);
        
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