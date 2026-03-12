use clap::{Parser, Subcommand};
use bitcoincore_rpc::RpcApi;
use dust_cleaner::{connect_to_node, classify_utxos};

#[derive(Parser)]
#[command(
    name = "dust-cleaner",
    about = "Detect and sweep dust attack UTXOs from your Bitcoin wallet"
)]
struct Cli {
    /// Bitcoin Core RPC URL
    #[arg(long, default_value = "http://127.0.0.1:18443")]
    rpc_url: String,

    /// Bitcoin Core RPC username
    #[arg(long)]
    rpc_user: String,

    /// Bitcoin Core RPC password
    #[arg(long)]
    rpc_pass: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan wallet for dust UTXOs
    Scan,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let client = connect_to_node(&cli.rpc_url, &cli.rpc_user, &cli.rpc_pass)?;

    match cli.command {
        Commands::Scan => {
            let utxos = client.list_unspent(None, None, None, None, None)?;
            println!("Found {} total UTXOs\n", utxos.len());

            let (dust_utxos, clean_utxos) = classify_utxos(utxos);

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
    }

    Ok(())
}