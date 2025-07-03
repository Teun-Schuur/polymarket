//! Price history and cryptocurrency price tracking

use std::{
    // collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use cli_log::*;

use crate::data::{SimpleOrder, CryptoPrice};
use crate::websocket::{CryptoWebSocket, CryptoSymbol};
use super::core::App;

pub fn update_price_history_if_needed(app: &mut App) {
    if should_update_price_history(app) {
        if let Some(ref mut orderbook) = app.orderbook {
            let weighted_mid_price = calculate_weighted_midpoint(&orderbook.bids, &orderbook.asks);
            if weighted_mid_price > 0.0 {
                orderbook.price_history.add_price(weighted_mid_price);
                app.last_price_history_update = Instant::now();
            }
        }
    }
}

pub fn should_update_price_history(app: &App) -> bool {
    app.last_price_history_update.elapsed() >= app.price_history_update_interval
}

pub fn calculate_weighted_midpoint(bids: &[SimpleOrder], asks: &[SimpleOrder]) -> f64 {
    let (best_bid, best_ask) = match (bids.first(), asks.first()) {
        (Some(bid), Some(ask)) => (bid.price, ask.price),
        _ => return 0.0,
    };
    
    if best_bid <= 0.0 || best_ask <= 0.0 {
        return 0.0;
    }
    
    let (bid_size, ask_size) = (bids.first().unwrap().size, asks.first().unwrap().size);
    
    if bid_size <= 0.0 && ask_size <= 0.0 {
        return (best_bid + best_ask) / 2.0;
    }
    
    // Weighted midpoint: side with more liquidity pulls price towards it
    let total_size = bid_size + ask_size;
    let ask_weight = ask_size / total_size;
    let bid_weight = bid_size / total_size;
    
    best_bid * ask_weight + best_ask * bid_weight
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
        info!("Crypto tracking for market '{}': {:?}", orderbook.market_question, symbols);
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
            // No relevant crypto symbols, stop all WebSockets
            info!("No relevant crypto symbols found, stopping all crypto WebSockets");
            app.crypto_websocket_active.clear();
            app.crypto_prices.clear();
            // Backward compatibility for Bitcoin
            app.bitcoin_price = None;
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
            
            // Backward compatibility for Bitcoin
            if matches!(active_symbol, CryptoSymbol::Bitcoin) {
                app.bitcoin_price = None;
            }
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
            
            // Backward compatibility for Bitcoin
            if matches!(symbol, CryptoSymbol::Bitcoin) {
                app.bitcoin_price = Some(crypto_price);
            }
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

impl App {
    pub fn should_update_price_history(&self) -> bool {
        should_update_price_history(self)
    }
    
    pub fn update_price_history_if_needed(&mut self) {
        update_price_history_if_needed(self);
    }
    
    pub fn should_show_bitcoin_chart(&self) -> bool {
        get_relevant_crypto_symbols(self)
            .map(|symbols| symbols.contains(&CryptoSymbol::Bitcoin))
            .unwrap_or(false)
    }
    
    pub fn start_bitcoin_websocket_if_needed(&mut self) {
        start_crypto_websockets_if_needed(self);
    }
    
    pub fn update_bitcoin_price_if_needed(&mut self) {
        update_crypto_prices_if_needed(self);
    }
    
    pub fn calculate_weighted_midpoint(bids: &[SimpleOrder], asks: &[SimpleOrder]) -> f64 {
        calculate_weighted_midpoint(bids, asks)
    }
}
