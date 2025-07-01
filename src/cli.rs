use clap::Parser;

#[derive(Parser)]
#[command(name = "polymarket-orderbook")]
#[command(about = "Real-time Polymarket orderbook viewer")]
pub struct Cli {
    /// Token ID to monitor (e.g., "21742633143463906290569050155826241533067272736897614950488156847949938836455")
    #[arg(short, long)]
    pub token_id: Option<String>,
    
    /// Update interval in seconds
    #[arg(short, long, default_value = "0.1")]
    pub interval: f64,
    
    /// Show top N orders per side
    #[arg(short, long, default_value = "30")]
    pub depth: usize,
    
    /// Private key environment variable name
    #[arg(long, default_value = "PK")]
    pub private_key_env: String,
}
