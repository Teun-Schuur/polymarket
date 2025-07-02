//! Price history and Bitcoin price tracking

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use cli_log::*;

use crate::data::SimpleOrder;
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

pub fn update_bitcoin_price_if_needed(app: &mut App) {
    // This will be called periodically to check if we need to start/stop Bitcoin tracking
    start_bitcoin_websocket_if_needed(app);
}

fn should_show_bitcoin_chart(app: &App) -> bool {
    if let Some(ref orderbook) = app.orderbook {
        orderbook.market_question.to_lowercase().contains("bitcoin") || 
        orderbook.market_question.to_lowercase().contains("btc")
    } else {
        false
    }
}

fn start_bitcoin_websocket_if_needed(app: &mut App) {
    if should_show_bitcoin_chart(app) && !app.bitcoin_websocket_active {
        info!("Starting Bitcoin WebSocket - market contains 'Bitcoin'");
        
        // Initialize Bitcoin price tracking with Arc<Mutex<>>
        app.bitcoin_price = Some(Arc::new(Mutex::new(crate::data::BitcoinPrice::new())));
        
        // Start the WebSocket in a separate thread
        start_bitcoin_websocket(app);
        app.bitcoin_websocket_active = true;
    } else if !should_show_bitcoin_chart(app) && app.bitcoin_websocket_active {
        info!("Stopping Bitcoin WebSocket - market no longer contains 'Bitcoin'");
        stop_bitcoin_websocket(app);
    }
}

fn start_bitcoin_websocket(app: &mut App) {
    use crate::websocket::BitcoinWebSocket;
    use std::thread;
    
    if let Some(ref bitcoin_price_arc) = app.bitcoin_price {
        let bitcoin_price = Arc::clone(bitcoin_price_arc);
        
        thread::spawn(move || {
            let mut btc_ws = BitcoinWebSocket::new();
            btc_ws.start();
            
            info!("Bitcoin WebSocket started");
            
            let mut last_price = 0.0;
            while btc_ws.is_running() {
                let current_price = btc_ws.get_price();
                if current_price > 0.0 && (current_price - last_price).abs() > 0.01 {
                    if let Ok(mut price_data) = bitcoin_price.lock() {
                        price_data.update_price(current_price);
                        last_price = current_price;
                    }
                }
                thread::sleep(Duration::from_millis(100));
            }
            
            info!("Bitcoin WebSocket ended");
        });
    }
}

fn stop_bitcoin_websocket(app: &mut App) {
    app.bitcoin_websocket_active = false;
    app.bitcoin_price = None;
}

impl App {
    pub fn should_update_price_history(&self) -> bool {
        should_update_price_history(self)
    }
    
    pub fn update_price_history_if_needed(&mut self) {
        update_price_history_if_needed(self);
    }
    
    pub fn should_show_bitcoin_chart(&self) -> bool {
        should_show_bitcoin_chart(self)
    }
    
    pub fn start_bitcoin_websocket_if_needed(&mut self) {
        start_bitcoin_websocket_if_needed(self);
    }
    
    pub fn update_bitcoin_price_if_needed(&mut self) {
        update_bitcoin_price_if_needed(self);
    }
    
    pub fn calculate_weighted_midpoint(bids: &[SimpleOrder], asks: &[SimpleOrder]) -> f64 {
        calculate_weighted_midpoint(bids, asks)
    }
}
