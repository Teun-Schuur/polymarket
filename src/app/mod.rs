// App module structure for better organization

pub mod types;
pub mod core;
pub mod navigation;
pub mod search;
pub mod websocket;
pub mod price_history;
pub mod input;
pub mod strategies;

// Re-export the main App struct and key types
pub use core::App;
pub use types::{SelectedTab, MarketSelectorTab};
