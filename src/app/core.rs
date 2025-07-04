//! Core application logic and initialization

use anyhow::Result;
use polymarket_rs_client::{ClobClient, Event, GammaMarket};
use rust_decimal::prelude::*;
use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use cli_log::*;

use crate::{
    bot::BotEngine,
    config::{POLYMARKET_HOST, POLYMARKET_GAMMA_HOST, POLYGON_CHAIN_ID, MAX_EVENTS, PRICE_HISTORY_UPDATE_INTERVAL_MS},
    data::{OrderBookData, PriceHistory, SimpleOrder}, 
    get_midpoint_from_slices
};
use crate::websocket::{PolymarketWebSocket, PolymarketWebSocketMessage};
use super::types::{SelectedTab, MarketSelectorTab};

pub struct App {
    // Core client and data
    pub client: ClobClient,
    pub orderbook: Option<OrderBookData>,
    pub markets: Vec<GammaMarket>,
    pub events: Vec<Event>,
    
    // Filtering and selection state
    pub filtered_markets: Vec<usize>, // Indices into markets vec for filtering/sorting
    pub filtered_events: Vec<usize>, // Indices into events vec for filtering/sorting
    pub selected_market: usize,
    pub selected_event: usize,
    pub selected_token: usize,
    
    // UI state
    pub market_scroll_offset: usize,
    pub event_scroll_offset: usize,
    pub token_scroll_offset: usize,
    pub show_market_selector: bool,
    pub show_event_market_selector: bool, // When true, shows markets within selected event
    pub show_token_selector: bool,
    pub market_selector_tab: MarketSelectorTab, // Tracks which tab is active in market selector
    pub needs_redraw: bool,
    pub selected_tab: SelectedTab,
    
    // Search functionality
    pub search_query: String,
    pub search_mode: bool,
    pub error_message: Option<String>,
    pub status_message: Option<String>, // For success/info messages
    pub status_message_time: Option<Instant>, // When the status message was set
    
    // Timing and updates
    pub last_update: Instant,
    pub last_orderbook_update: Instant,
    pub update_interval: Duration,
    pub depth: usize,
    
    // Price history data from API
    pub market_price_history: Option<polymarket_rs_client::PriceHistoryResponse>,
    pub last_price_history_update: Instant,
    pub price_history_update_interval: Duration,
    
    // WebSocket integration for real-time updates
    pub current_websocket: Option<PolymarketWebSocket>,
    pub websocket_updates: Arc<Mutex<Vec<PolymarketWebSocketMessage>>>,
    pub last_websocket_check: Instant,
    pub websocket_reconnect_attempts: u32,
    pub last_websocket_attempt: Instant,
    
    // Multi-crypto price tracking
    pub crypto_prices: std::collections::HashMap<crate::websocket::CryptoSymbol, Arc<Mutex<crate::data::CryptoPrice>>>,
    pub crypto_websocket_active: std::collections::HashMap<crate::websocket::CryptoSymbol, bool>,
    
    // Bot engine for strategy execution
    pub bot_engine: BotEngine,
    pub show_strategy_selector: bool,
    pub show_strategy_runner: bool,
    pub selected_strategy: usize,
    pub strategy_selection_mode: bool, // True when we're picking markets/events for a strategy
}

impl App {
    pub async fn new(interval: f64, depth: usize, private_key_env: &str) -> Result<Self> {
        let private_key = env::var(private_key_env)
            .map_err(|_| anyhow::anyhow!(
                "Private key not found in environment variable '{}'. Please set it in your .env file or environment.", 
                private_key_env
            ))?;

        let mut client = ClobClient::with_l1_headers(POLYMARKET_HOST, POLYMARKET_GAMMA_HOST, &private_key, POLYGON_CHAIN_ID);
        
        // Create or derive API key
        let nonce = None;
        let keys = client.create_or_derive_api_key(nonce).await.unwrap();
        
        client.set_api_creds(keys);
        
        Ok(Self {
            client,
            orderbook: None,
            markets: Vec::new(),
            events: Vec::new(),
            filtered_markets: Vec::new(),
            filtered_events: Vec::new(),
            selected_market: 0,
            selected_event: 0,
            selected_token: 0,
            market_scroll_offset: 0,
            event_scroll_offset: 0,
            token_scroll_offset: 0,
            show_market_selector: true,
            show_event_market_selector: false,
            show_token_selector: false,
            market_selector_tab: MarketSelectorTab::AllMarkets,
            last_update: Instant::now(),
            last_orderbook_update: Instant::now(),
            update_interval: Duration::from_secs_f64(interval),
            depth,
            error_message: None,
            status_message: None,
            status_message_time: None,
            search_query: String::new(),
            search_mode: false,
            needs_redraw: true,
            selected_tab: SelectedTab::Orderbook,
            market_price_history: None,
            current_websocket: None,
            websocket_updates: Arc::new(Mutex::new(Vec::new())),
            last_websocket_check: Instant::now(),
            websocket_reconnect_attempts: 0,
            last_websocket_attempt: Instant::now(),
            last_price_history_update: Instant::now(),
            price_history_update_interval: Duration::from_millis(PRICE_HISTORY_UPDATE_INTERVAL_MS),
            crypto_prices: HashMap::new(),
            crypto_websocket_active: HashMap::new(),
            
            // Bot engine
            bot_engine: BotEngine::new(),
            show_strategy_selector: false,
            show_strategy_runner: false,
            selected_strategy: 0,
            strategy_selection_mode: false,
        })
    }

    pub async fn load_markets(&mut self) -> Result<()> {
        self.events.clear();
        self.markets.clear();

        let mut index = 0;
        loop {
            match self.client.get_gamma_events(Some(index), Some(500)).await {
                Ok(events) => {
                    // Get the length before moving the events
                    let num_events = events.len();
                    
                    // Extend events with the new data
                    self.events.extend(events);
                    let tot_events = self.events.len();
                    
                    // Check if we have more events to load
                    if num_events < 500 || tot_events >= MAX_EVENTS {
                        info!("Reached end of gamma events pagination or max limit reached: {tot_events} total events loaded");
                        break;
                    }
                    
                    index += 500; // Increment index for next batch

                }
                Err(e) => {
                    warn!("Failed to load gamma events: {e}");
                    self.error_message = Some(format!("Failed to load gamma events: {e}"));
                }
            }
        }
        
        // prune empty events
        self.prune_empty_events();
        // Sort events by volume
        self.events.sort_by(|a, b| b.volume.unwrap_or(0.0)
            .partial_cmp(&a.volume.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal));
        
        // Load all markets from the events
        for event in &self.events {
            if let Some(markets) = &event.markets {
                self.markets.extend(markets.clone());
            }
        }

        // Sort markets by volume
        self.markets.sort_by(|a, b| b.volume.unwrap_or(Decimal::ZERO)
            .partial_cmp(&a.volume.unwrap_or(Decimal::ZERO))
            .unwrap_or(std::cmp::Ordering::Equal));
        
        // Initialize filtered markets and events with all indices
        self.update_filtered_markets();
        self.update_filtered_events();
        
        // Reset selection if it's out of bounds
        if self.selected_market >= self.filtered_markets.len() {
            self.selected_market = 0;
            self.market_scroll_offset = 0;
        }
        
        if self.selected_event >= self.filtered_events.len() {
            self.selected_event = 0;
            self.event_scroll_offset = 0;
        }
        
        self.error_message = None;        
        info!("Successfully loaded {} markets total", self.markets.len());
        Ok(())
    }

        
    pub fn prune_empty_events(&mut self) {
        // Remove markets inside events that are no longer active or are closed
        self.events.retain_mut(|event| {
            if let Some(markets) = &mut event.markets {
                // Filter out markets that are closed or inactive
                markets.retain(|m| m.active && !m.closed && m.uma_resolution_statuses.as_ref().unwrap_or(&Vec::new()).is_empty());
                
                // Keep the event if it has any active markets left
                !markets.is_empty()
            } else {
                false // If no markets, remove the event
            }
        });
    }

    pub fn update_filtered_markets(&mut self) {
        self.filtered_markets.clear();
        
        if self.search_query.is_empty() {
            // No filter, show all markets
            self.filtered_markets = (0..self.markets.len()).collect();
        } else {
            // Filter markets based on search query
            let query = self.search_query.to_lowercase();
            for (i, market) in self.markets.iter().enumerate() {
                if market.question.to_lowercase().contains(&query) {
                    self.filtered_markets.push(i);
                }
            }
        }
        
        // Reset selection if it's out of bounds
        if self.selected_market >= self.filtered_markets.len() && !self.filtered_markets.is_empty() {
            self.selected_market = 0;
            self.market_scroll_offset = 0;
        }
    }

    pub fn update_filtered_events(&mut self) {
        self.filtered_events.clear();
        
        if self.search_query.is_empty() {
            // No filter, show events with more than 1 market
            for (i, event) in self.events.iter().enumerate() {
                let market_count = event.markets.as_ref().map(|m| m.len()).unwrap_or(0);
                if market_count > 1 {
                    self.filtered_events.push(i);
                }
            }
        } else {
            // Filter events based on search query and only show events with more than 1 market
            let query = self.search_query.to_lowercase();
            for (i, event) in self.events.iter().enumerate() {
                let market_count = event.markets.as_ref().map(|m| m.len()).unwrap_or(0);
                if market_count > 1 && 
                   (event.title.to_lowercase().contains(&query) ||
                    event.description.to_lowercase().contains(&query)) {
                    self.filtered_events.push(i);
                }
            }
        }
        
        // Reset selection if it's out of bounds
        if self.selected_event >= self.filtered_events.len() && !self.filtered_events.is_empty() {
            self.selected_event = 0;
            self.event_scroll_offset = 0;
        }
    }
    
    pub async fn load_orderbook(&mut self, token_id: &str) -> Result<()> {
        // Fetch price history for the market
        match self.client.get_price_history(token_id, "max", 60).await {
            Ok(price_history) => {
                // Store the price history for the tab display
                self.market_price_history = Some(price_history);
                info!("Loaded price history for token ID: {token_id}");
            }
            Err(e) => {
                warn!("Failed to load price history: {e}");
                self.market_price_history = None;
                // Don't return early - continue to load orderbook
            }
        }

        match self.client.get_order_book(token_id).await {
            Ok(book) => {
                // Find market details
                let market_question = self.markets
                    .iter()
                    .find(|m| m.token_ids.iter().any(|t| t == token_id))
                    .and_then(|m| {
                        m.token_ids.iter()
                            .position(|t| t == token_id)
                            .and_then(|index| m.outcomes.get(index))
                            .map(|outcome| format!("{} - {}", m.question, outcome))
                    })
                    .unwrap_or_else(|| token_id.to_string());

                // Convert API orders to simple orders
                let mut bids = Vec::new();
                let mut asks = Vec::new();
                
                // Convert bids
                for bid in &book.bids {
                    bids.push(SimpleOrder::new(
                        bid.price.to_f64().unwrap_or(0.0), 
                        bid.size.to_f64().unwrap_or(0.0)
                    ));
                }
                
                // Convert asks
                for ask in &book.asks {
                    asks.push(SimpleOrder::new(
                        ask.price.to_f64().unwrap_or(0.0), 
                        ask.size.to_f64().unwrap_or(0.0)
                    ));
                }
                
                // Sort and limit orders
                bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
                bids.truncate(self.depth);
                
                asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));
                asks.truncate(self.depth);

                // Get tick size from API
                let tick_size = self.get_tick_size_for_token(token_id).await;

                // Preserve existing price history if updating the same token
                let price_history = if let Some(ref existing_orderbook) = self.orderbook {
                    if existing_orderbook.token_id == token_id {
                        // Keep existing price history and add new midpoint
                        let mut history = existing_orderbook.price_history.clone();
                        let mid_price = get_midpoint_from_slices(&bids, &asks);
                        history.add_price(mid_price);
                        history
                    } else {
                        // Different token, start fresh
                        PriceHistory::new(500)
                    }
                } else {
                    // No existing orderbook, start fresh
                    PriceHistory::new(500)
                };

                self.orderbook = Some(OrderBookData {
                    token_id: token_id.to_string(),
                    market_question,
                    bids,
                    asks,
                    tick_size,
                    last_updated: chrono::Utc::now(),
                    chart_center_price: None,
                    chart_needs_recentering: true,
                    price_history,
                });
                self.error_message = None;
                self.last_update = Instant::now();
                self.needs_redraw = true;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load orderbook: {e}"));
                self.needs_redraw = true;
            }
        }

        Ok(())
    }

    pub async fn update(&mut self) -> Result<()> {
        // Clear old status messages
        self.clear_old_status_message();
        
        // Process WebSocket updates for real-time data
        if let Err(e) = self.process_websocket_updates() {
            warn!("WebSocket update failed: {e}");
        }
        
        // Clean up expired highlights
        if let Some(ref mut orderbook) = self.orderbook {
            for order in &mut orderbook.bids {
                order.clear_highlight_if_expired();
            }
            for order in &mut orderbook.asks {
                order.clear_highlight_if_expired();
            }
        }
        
        // Update orderbook via API if needed
        if self.should_update_orderbook_via_api() {
            if let Some(orderbook) = &self.orderbook {
                let token_id = orderbook.token_id.clone();
                if let Err(e) = self.load_orderbook(&token_id).await {
                    warn!("Failed to load orderbook via API: {e}");
                }
                self.last_orderbook_update = Instant::now();
                self.needs_redraw = true;
            }
        }

        super::price_history::update_price_history_if_needed(self);
        super::price_history::update_crypto_prices_if_needed(self);
        
        // Process orderbook with bot engine
        if let Some(ref orderbook) = self.orderbook {
            if let Err(e) = self.bot_engine.process_orderbook(orderbook) {
                warn!("Bot engine processing failed: {e}");
            }
        }
        
        Ok(())
    }
    
    fn should_update_orderbook_via_api(&self) -> bool {
        if self.last_orderbook_update.elapsed() < self.update_interval {
            return false;
        }
        
        // If we have an active WebSocket, we don't need to poll the API
        self.current_websocket.is_none()
    }

    // Helper methods that will need to be implemented
    async fn get_tick_size_for_token(&self, token_id: &str) -> f64 {
        match self.client.get_tick_size(token_id).await {
            Ok(tick_size) => tick_size.to_f64().unwrap_or(0.0001),
            Err(_) => 0.0001, // Default tick size for prediction markets
        }
    }

    fn process_websocket_updates(&mut self) -> Result<()> {
        // Delegate to websocket module
        super::websocket::process_websocket_updates(self)
    }

    pub fn update_price_history_if_needed(&mut self) {
        // This will be called periodically to update price history
        if super::price_history::should_update_price_history(self) {
            if let Some(ref mut orderbook) = self.orderbook {
                let midpoint = orderbook.get_midpoint();
                orderbook.price_history.add_price(midpoint);
                self.last_price_history_update = Instant::now();
            }
        }
    }

    pub fn set_status_message(&mut self, message: String) {
        self.status_message = Some(message);
        self.status_message_time = Some(Instant::now());
        self.needs_redraw = true;
    }
    
    pub fn clear_old_status_message(&mut self) {
        if let Some(time) = self.status_message_time {
            if time.elapsed() > Duration::from_secs(3) { // Clear after 3 seconds
                self.status_message = None;
                self.status_message_time = None;
                self.needs_redraw = true;
            }
        }
    }
}
