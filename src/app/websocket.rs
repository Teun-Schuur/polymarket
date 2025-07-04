//! WebSocket integration for real-time updates

use anyhow::Result;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use cli_log::*;

use crate::{
    config::{WS_MAX_ATTEMPTS, WS_RECONNECT_DELAY_SECS},
    data::{OrderBookData, SimpleOrder}
};
use crate::websocket::{
    BookMessage, LastTradePriceMessage, PolymarketWebSocket, PolymarketWebSocketMessage,
    PriceChangeMessage, MessageCallback,
};
use super::core::App;

pub fn process_websocket_updates(app: &mut App) -> Result<()> {
    if app.last_websocket_check.elapsed() < Duration::from_millis(50) {
        return Ok(());
    }
    app.last_websocket_check = Instant::now();
    
    // Check WebSocket connection health
    check_websocket_health(app);
    
    // Process pending updates
    let updates = match app.websocket_updates.try_lock() {
        Ok(mut guard) if !guard.is_empty() => std::mem::take(&mut *guard),
        _ => return Ok(()),
    };
    
    for update in updates {
        apply_websocket_update(app, update)?;
    }
    
    Ok(())
}

fn check_websocket_health(app: &mut App) {
    if let Some(ref ws) = app.current_websocket {
        if ws.thread_handle.is_finished() {
            warn!("WebSocket thread terminated, reconnecting");
            app.current_websocket = None;
            if let Some(ref orderbook) = app.orderbook {
                try_reconnect_websocket(app, &orderbook.token_id.clone());
            }
            app.needs_redraw = true;
        }
    }
}

fn apply_websocket_update(app: &mut App, update: PolymarketWebSocketMessage) -> Result<()> {
    let orderbook = match &mut app.orderbook {
        Some(ob) => ob,
        None => return Ok(()),
    };
    
    let asset_matches = match &update {
        PolymarketWebSocketMessage::Book(msg) => msg.asset_id == orderbook.token_id,
        PolymarketWebSocketMessage::PriceChange(msg) => msg.asset_id == orderbook.token_id,
        PolymarketWebSocketMessage::LastTradePrice(msg) => msg.asset_id == orderbook.token_id,
        PolymarketWebSocketMessage::TickSizeChange(msg) => msg.asset_id == orderbook.token_id,
        PolymarketWebSocketMessage::Unknown(_) => false,
    };
    
    if !asset_matches {
        return Ok(());
    }
    
    match update {
        PolymarketWebSocketMessage::Book(book_msg) => {
            apply_book_update_static(orderbook, &book_msg, app.depth)?;
        }
        PolymarketWebSocketMessage::PriceChange(price_msg) => {
            apply_price_changes_static(orderbook, &price_msg, app.depth)?;
        }
        PolymarketWebSocketMessage::LastTradePrice(trade_msg) => {
            apply_trade_update_static(orderbook, &trade_msg)?;
        }
        PolymarketWebSocketMessage::TickSizeChange(tick_msg) => {
            if let Ok(new_tick_size) = tick_msg.new_tick_size.parse::<f64>() {
                orderbook.tick_size = new_tick_size;
            }
        }
        PolymarketWebSocketMessage::Unknown(_) => return Ok(()),
    }
    
    app.needs_redraw = true;
    Ok(())
}

fn apply_book_update_static(orderbook: &mut OrderBookData, book_msg: &BookMessage, depth: usize) -> Result<()> {
    // Convert WebSocket book data to our SimpleOrder format
    let mut new_bids = Vec::new();
    for bid in &book_msg.bids {
        if let (Ok(price), Ok(size)) = (bid.price.parse::<f64>(), bid.size.parse::<f64>()) {
            new_bids.push(SimpleOrder::new(price, size));
        }
    }
    
    let mut new_asks = Vec::new();
    for ask in &book_msg.asks {
        if let (Ok(price), Ok(size)) = (ask.price.parse::<f64>(), ask.size.parse::<f64>()) {
            new_asks.push(SimpleOrder::new(price, size));
        }
    }
    
    // Sort and limit orders
    new_bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
    new_bids.truncate(depth);
    
    new_asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));
    new_asks.truncate(depth);
    
    // Update orderbook
    orderbook.bids = new_bids;
    orderbook.asks = new_asks;
    orderbook.last_updated = chrono::Utc::now();
    orderbook.chart_needs_recentering = true; // Re-center chart on updates
    
    // Recalculate market stats and update price history
    orderbook.price_history.add_price(orderbook.get_midpoint());
    
    Ok(())
}

fn apply_price_changes_static(orderbook: &mut OrderBookData, price_msg: &PriceChangeMessage, depth: usize) -> Result<()> {
    for change in &price_msg.changes {
        let (price, size) = match (change.price.parse::<f64>(), change.size.parse::<f64>()) {
            (Ok(p), Ok(s)) => (p, s),
            _ => continue,
        };
        
        let orders = match change.side.to_lowercase().as_str() {
            "bid" | "bids" | "buy" => &mut orderbook.bids,
            "ask" | "asks" | "sell" => &mut orderbook.asks,
            _ => {
                warn!("Unknown order side: '{}'", change.side);
                continue;
            }
        };
        
        // Update or remove existing order
        if let Some(existing_order) = orders.iter_mut().find(|o| (o.price - price).abs() < 0.0001) {
            if size == 0.0 {
                orders.retain(|o| (o.price - price).abs() >= 0.0001);
            } else {
                existing_order.update_size(size);
            }
        } else if size > 0.0 {
            orders.push(SimpleOrder::new(price, size));
        }
    }
    
    // Re-sort and limit orders
    orderbook.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
    orderbook.bids.truncate(depth);
    orderbook.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));
    orderbook.asks.truncate(depth);
    
    orderbook.last_updated = chrono::Utc::now();
    orderbook.chart_needs_recentering = true;

    orderbook.price_history.add_price(orderbook.get_midpoint());    
    Ok(())
}

fn apply_trade_update_static(orderbook: &mut OrderBookData, _trade_msg: &LastTradePriceMessage) -> Result<()> {
    orderbook.last_updated = chrono::Utc::now();
    orderbook.price_history.add_price(orderbook.get_midpoint());
    Ok(())
}

fn try_reconnect_websocket(app: &mut App, token_id: &str) {
    if app.websocket_reconnect_attempts >= WS_MAX_ATTEMPTS 
        || app.last_websocket_attempt.elapsed() < Duration::from_secs(WS_RECONNECT_DELAY_SECS) {
        return;
    }
    
    info!("Reconnecting WebSocket (attempt {}/{})", 
          app.websocket_reconnect_attempts + 1, WS_MAX_ATTEMPTS);
    
    app.websocket_reconnect_attempts += 1;
    app.last_websocket_attempt = Instant::now();
    start_websocket_for_token(app, token_id);
}

fn start_websocket_for_token(app: &mut App, token_id: &str) {
    info!("Starting WebSocket connection for token: {token_id}");
    
    // Close existing WebSocket if any
    if app.current_websocket.is_some() {
        info!("Closing existing WebSocket connection");
        app.current_websocket = None;
    }
    
    let updates_arc: Arc<Mutex<Vec<PolymarketWebSocketMessage>>> = Arc::clone(&app.websocket_updates);
    let token_id_owned = token_id.to_string();
    
    let callback: MessageCallback = Box::new(move |msg| {
        let message_matches = match &msg {
            PolymarketWebSocketMessage::Book(book_msg) => book_msg.asset_id == token_id_owned,
            PolymarketWebSocketMessage::PriceChange(price_msg) => price_msg.asset_id == token_id_owned,
            PolymarketWebSocketMessage::TickSizeChange(tick_msg) => tick_msg.asset_id == token_id_owned,
            PolymarketWebSocketMessage::LastTradePrice(trade_msg) => trade_msg.asset_id == token_id_owned,
            PolymarketWebSocketMessage::Unknown(_) => false,
        };
        
        if message_matches {
            if let Ok(mut updates) = updates_arc.lock() {
                updates.push(msg);
                // Keep only recent updates to avoid memory issues
                if updates.len() > 50 {
                    updates.drain(0..25);
                }
            }
        }
    });
    
    app.current_websocket = Some(PolymarketWebSocket::connect(
        "market".into(),
        None,
        vec![token_id.to_string()],
        callback,
    ));
    
    info!("WebSocket started for token: {token_id}");
}

impl App {
    pub fn start_websocket_for_token(&mut self, token_id: &str) {
        start_websocket_for_token(self, token_id);
    }
    
    pub fn try_reconnect_websocket(&mut self, token_id: &str) {
        try_reconnect_websocket(self, token_id);
    }
    
    pub fn reset_websocket_reconnect_counter(&mut self) {
        self.websocket_reconnect_attempts = 0;
    }
}
