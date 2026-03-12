use clap::Parser;
use dust_cleaner::cli::{Cli, Commands};
use dust_cleaner::{analyzer, rpc, scanner};

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
            println!("Sweep coming soon...");
        }
    }

    Ok(())
}