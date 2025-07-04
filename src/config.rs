// Configuration constants for the application

/// API endpoints
pub const POLYMARKET_HOST: &str = "https://clob.polymarket.com";
pub const POLYMARKET_GAMMA_HOST: &str = "https://gamma-api.polymarket.com";

/// Network settings
pub const POLYGON_CHAIN_ID: u64 = 137;

/// Application limits
pub const MAX_EVENTS: usize = 5000; // Limit to prevent excessive memory usage
pub const MAX_PRICE_HISTORY_POINTS: usize = 300; // Store last 300 points

/// Update intervals (in milliseconds)
pub const TICK_RATE_MS: u64 = 1;
pub const DATA_UPDATE_RATE_MS: u64 = 50;
pub const UI_UPDATE_RATE_MS: u64 = 1000;
pub const PRICE_HISTORY_UPDATE_INTERVAL_MS: u64 = 60_000; // 1 minute

/// WebSocket settings
pub const WS_MAX_ATTEMPTS: u32 = 20;
pub const WS_RECONNECT_DELAY_SECS: u64 = 10;

/// UI settings
pub const HIGHLIGHT_DURATION_MS: u128 = 1000; // Highlight changes for 1 second
pub const CHART_NUM_DATES: u32 = 5;

/// Default CLI values
pub const DEFAULT_UPDATE_INTERVAL: f64 = 0.1;
pub const DEFAULT_ORDERBOOK_DEPTH: usize = 30;
pub const DEFAULT_PRIVATE_KEY_ENV: &str = "PK";
