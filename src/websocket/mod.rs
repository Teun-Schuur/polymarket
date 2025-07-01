pub mod clob;
pub mod bitcoin;

// Re-export commonly used types
pub use clob::{
    BookMessage, 
    LastTradePriceMessage, 
    PolymarketWebSocket, 
    PolymarketWebSocketMessage,
    PriceChangeMessage,
    StructuredMessageCallback,
};

pub use bitcoin::BitcoinWebSocket;
