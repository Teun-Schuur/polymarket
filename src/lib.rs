// Library exports for the polymarket orderbook viewer
pub mod app;
pub mod bot;
pub mod cli;
pub mod config;
pub mod data;
pub mod ui;
pub mod websocket;
pub mod utils;

// Re-export commonly used types
pub use app::{App, MarketSelectorTab};
pub use bot::{BotEngine, Strategy, StrategyType};
pub use cli::Cli;
pub use data::{MarketInfo, OrderBookData, SimpleOrder, TokenInfo, BitcoinPrice};
pub use ui::render_ui;
pub use websocket::{PolymarketWebSocket, PolymarketWebSocketMessage, MessageCallback};
pub use utils::*;
