// Library exports for the polymarket orderbook viewer
pub mod app;
pub mod cli;
pub mod data;
pub mod ui;
pub mod websocket;

// Re-export commonly used types
pub use app::App;
pub use cli::Cli;
pub use data::{MarketInfo, MarketStats, OrderBookData, SimpleOrder, TokenInfo, BitcoinPrice};
pub use ui::render_ui;
pub use websocket::{PolymarketWebSocket, PolymarketWebSocketMessage, StructuredMessageCallback, BitcoinWebSocket};

