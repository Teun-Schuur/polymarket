// UI module organization
pub mod layout;
pub mod selectors;
pub mod orderbook;
pub mod charts;
pub mod components;
pub mod strategies;

// Re-export the main UI function
pub use layout::render_ui;
