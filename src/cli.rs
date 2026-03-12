use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "dust-cleaner",
    about = "Detect and sweep dust attack UTXOs from your Bitcoin wallet"
)]
pub struct Cli {
    /// Bitcoin Core RPC URL
    #[arg(long, default_value = "http://127.0.0.1:18443")]
    pub rpc_url: String,

    /// Bitcoin Core RPC username
    #[arg(long)]
    pub rpc_user: String,

    /// Bitcoin Core RPC password
    #[arg(long)]
    pub rpc_pass: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan wallet for dust UTXOs
    Scan,
    /// Create a PSBT sweeping all dust UTXOs to miner fees
    Sweep,
}