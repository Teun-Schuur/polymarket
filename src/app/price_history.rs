//! Price history and cryptocurrency price tracking

use std::{
    // collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use cli_log::*;

use crate::data::{CryptoPrice};
use crate::websocket::{CryptoWebSocket, CryptoSymbol};
use super::core::App;

pub fn update_price_history_if_needed(app: &mut App) {
    if should_update_price_history(app) {
        if let Some(ref mut orderbook) = app.orderbook {
            let midpoint = orderbook.get_midpoint();
            orderbook.price_history.add_price(midpoint);
            app.last_price_history_update = Instant::now();
        }
    }
}

pub fn should_update_price_history(app: &App) -> bool {
    app.last_price_history_update.elapsed() >= app.price_history_update_interval
}

pub fn update_crypto_prices_if_needed(app: &mut App) {
    // This will be called periodically to check if we need to start/stop crypto tracking
    start_crypto_websockets_if_needed(app);
}

fn get_relevant_crypto_symbols(app: &App) -> Option<Vec<CryptoSymbol>> {
    let mut symbols = Vec::new();
    
    // Only track crypto if we have an active orderbook
    if let Some(ref orderbook) = app.orderbook {
        let question_lower = orderbook.market_question.to_lowercase();
        
        // Only add ONE crypto symbol per market to avoid unnecessary subscriptions
        // Priority: Bitcoin > Ethereum > Solana (most commonly referenced)
        if question_lower.contains("bitcoin") || question_lower.contains("btc") {
            symbols.push(CryptoSymbol::Bitcoin);
        } else if question_lower.contains("ethereum") || question_lower.contains("eth") {
            symbols.push(CryptoSymbol::Ethereum);
        } else if question_lower.contains("solana") || question_lower.contains("sol") {
            symbols.push(CryptoSymbol::Solana);
        } else{
            return None;
        }        
    } else {
        // No orderbook selected, don't track any crypto
        // debug!("No orderbook selected, stopping all crypto tracking");
    }
    
    Some(symbols)
}

fn start_crypto_websockets_if_needed(app: &mut App) {
    // Get the currently relevant crypto symbols based on the active orderbook
    let relevant_symbols = match get_relevant_crypto_symbols(app) {
        Some(symbols) => symbols,
        None => {
            if app.crypto_websocket_active.is_empty() {
                // No active crypto WebSockets, nothing to stop
                return;
            }
            app.crypto_websocket_active.clear();
            app.crypto_prices.clear();
            return;
        }
    };
    
    // Stop all currently active crypto WebSockets that are no longer relevant
    let active_symbols: Vec<_> = app.crypto_websocket_active.keys().cloned().collect();
    for active_symbol in active_symbols {
        if !relevant_symbols.contains(&active_symbol) {
            info!("Stopping {} WebSocket - no longer relevant for current market", active_symbol.name());
            app.crypto_websocket_active.remove(&active_symbol);
            app.crypto_prices.remove(&active_symbol);
        }
    }
    
    // Start WebSockets for newly relevant symbols
    for symbol in relevant_symbols {
        if !app.crypto_websocket_active.get(&symbol).unwrap_or(&false) {
            info!("Starting {} WebSocket for market tracking", symbol.name());
            
            // Initialize crypto price tracking
            let crypto_price = Arc::new(Mutex::new(CryptoPrice::new(symbol.symbol().to_string())));
            app.crypto_prices.insert(symbol.clone(), crypto_price.clone());
            
            // Start the WebSocket in a separate thread
            start_crypto_websocket(symbol.clone(), crypto_price.clone());
            app.crypto_websocket_active.insert(symbol.clone(), true);
        }
    }
}

fn start_crypto_websocket(symbol: CryptoSymbol, price_arc: Arc<Mutex<CryptoPrice>>) {
    use std::thread;
    
    thread::spawn(move || {
        let mut crypto_ws = CryptoWebSocket::new();
        crypto_ws.start_single(symbol.clone());  // Use start_single for individual symbol
        
        info!("{} WebSocket started", symbol.name());
        
        let mut last_price = 0.0;
        while crypto_ws.is_running() {
            let current_price = crypto_ws.get_price(&symbol);
            if current_price > 0.0 && (current_price - last_price).abs() > 0.01 {
                if let Ok(mut price_data) = price_arc.lock() {
                    price_data.update_price(current_price);
                    last_price = current_price;
                    debug!("{} price updated: ${:.2}", symbol.name(), current_price);
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
        
        info!("{} WebSocket ended", symbol.name());
    });
}