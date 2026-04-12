use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "dust-cleaner",
    about = "Detect and sweep dust attack UTXOs from your Bitcoin wallet"
)]
pub struct Cli {
    /// Bitcoin Core RPC URL
    #[arg(long, default_value = "http://127.0.0.1:18443", env = "DUST_RPC_URL")]
    pub rpc_url: String,

    /// Bitcoin Core RPC username
    #[arg(long, env = "DUST_RPC_USER")]
    pub rpc_user: String,

    /// Bitcoin Core RPC password
    #[arg(long, env = "DUST_RPC_PASS")]
    pub rpc_pass: String,

    /// Dust threshold in sats. If omitted, uses per-script-type thresholds
    #[arg(long, env = "DUST_THRESHOLD")]
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

        /// Sweep method: consolidate (default) or op-return (burn to fees)
        #[arg(long, value_enum, default_value = "consolidate")]
        method: SweepMethod,
    },
}
