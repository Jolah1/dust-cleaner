use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "dust-cleaner",
    about = "Detect and sweep dust attack UTXOs from your Bitcoin wallet"
)]
pub struct Cli {
    /// Bitcoin Core RPC URL
    #[arg(long, default_value = "http://127.0.0.1:18443")]
    pub rpc_url: String,

    #[arg(long)]
    pub rpc_user: String,


    #[arg(long)]
    pub rpc_pass: String,

    /// Custom dust threshold in sats. If omitted, uses per-script-type thresholds
    #[arg(long)]
    pub threshold: Option<u64>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum SweepMethod {
    /// Consolidate dust into a fresh wallet address
    Consolidate,
    /// Burn dust to miner fees via OP_RETURN (more private)
    OpReturn,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan wallet for dust UTXOs
    Scan,
    /// Create a PSBT sweeping all dust UTXOs
    Sweep {
        /// Preview the sweep without creating a PSBT
        #[arg(long, default_value = "false")]
        dry_run: bool,

        /// Sweep method: op-return (burn to fees)
        #[arg(long, value_enum, default_value = "consolidate")]
        method: SweepMethod,
    },
}