use clap::Parser;
use crate::config::{DEFAULT_UPDATE_INTERVAL, DEFAULT_ORDERBOOK_DEPTH, DEFAULT_PRIVATE_KEY_ENV};

#[derive(Parser)]
#[command(name = "polymarket-orderbook")]
#[command(about = "Real-time Polymarket orderbook viewer")]
pub struct Cli {
    /// Token ID to monitor (e.g., "21742633143463906290569050155826241533067272736897614950488156847949938836455")
    #[arg(short, long)]
    pub token_id: Option<String>,
    
    /// Update interval in seconds
    #[arg(short, long, default_value_t = DEFAULT_UPDATE_INTERVAL)]
    pub interval: f64,
    
    /// Show top N orders per side
    #[arg(short, long, default_value_t = DEFAULT_ORDERBOOK_DEPTH)]
    pub depth: usize,
    
    /// Private key environment variable name
    #[arg(long, default_value = DEFAULT_PRIVATE_KEY_ENV)]
    pub private_key_env: String,
}
