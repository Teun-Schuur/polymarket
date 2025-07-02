//! Core application logic and initialization

use anyhow::Result;
use polymarket_rs_client::{ClobClient, Event, Market};
use rust_decimal::prelude::*;
use std::{
    env,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use cli_log::*;

use crate::data::{OrderBookData, SimpleOrder, PriceHistory};
use crate::websocket::{PolymarketWebSocket, PolymarketWebSocketMessage};
use super::types::{SelectedTab, MarketSelectorTab};

pub struct App {
    // Core client and data
    pub client: ClobClient,
    pub orderbook: Option<OrderBookData>,
    pub markets: Vec<Market>,
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
    
    // Bitcoin price tracking
    pub bitcoin_price: Option<Arc<Mutex<crate::data::BitcoinPrice>>>,
    pub bitcoin_websocket_active: bool,
}

impl App {
    pub async fn new(interval: f64, depth: usize, private_key_env: &str) -> Result<Self> {
        let private_key = env::var(private_key_env)
            .map_err(|_| anyhow::anyhow!(
                "Private key not found in environment variable '{}'. Please set it in your .env file or environment.", 
                private_key_env
            ))?;

        const HOST: &str = "https://clob.polymarket.com";
        const HOST_GAMMA: &str = "https://gamma-api.polymarket.com";

        const POLYGON: u64 = 137;
        
        let mut client = ClobClient::with_l1_headers(HOST, HOST_GAMMA, &private_key, POLYGON);
        
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
            price_history_update_interval: Duration::from_secs(1),
            bitcoin_price: None,
            bitcoin_websocket_active: false,
        })
    }

    pub async fn load_markets(&mut self) -> Result<()> {
        self.markets.clear();
        let mut cursor = String::new(); // Start with empty cursor
        const MAX_MARKETS: usize = 5000; // Limit to prevent excessive memory usage
        
        loop {
            // Make request with current cursor
            let cursor_param = if cursor.is_empty() { None } else { Some(cursor.as_str()) };
            
            match self.client.get_sampling_markets(cursor_param).await {
                Ok(response) => {                    
                    // Extend markets with the new data
                    self.markets.extend(response.data);

                    // Check if we have a next_cursor to continue
                    if let Some(next_cursor) = response.next_cursor.as_deref() {
                        // Check if we've reached the end (cursor is "LTE=" or empty)
                        if next_cursor == "LTE=" || next_cursor.is_empty() {
                            info!("Reached end of markets pagination");
                            break;
                        }
                        
                        // Check if we've hit our limit
                        let total_loaded = self.markets.len();
                        if total_loaded >= MAX_MARKETS {
                            info!("Reached maximum market limit ({MAX_MARKETS}), stopping pagination");
                            break;
                        }
                        
                        cursor = next_cursor.to_string();
                        info!("Loading next batch of markets... (total so far: {total_loaded})");
                    } else {
                        // No next_cursor found, we're done
                        break;
                    }
                }
                Err(e) => {
                    // If this is the first request, try with fallback
                    if cursor.is_empty() {
                        warn!("Failed to load markets without cursor, trying with '20' parameter");
                        match self.client.get_sampling_markets(Some("20")).await {
                            Ok(sampling_markets) => {
                                // Ignore the result and continue the loop or just return Ok(())
                                self.markets.extend(sampling_markets.data);
                                return Ok(());
                            }
                            Err(e2) => {
                                self.error_message = Some(format!("Failed to load markets: {e2}"));
                                return Ok(());
                            }
                        }
                    } else {
                        // Error on subsequent request, stop pagination
                        warn!("Error during pagination: {e:?}");
                        break;
                    }
                }
            }
        }

        let mut index = 0;
        const MAX_EVENTS: usize = 5000; // Limit to prevent excessive memory usage
        loop {
            match self.client.get_gamma_events(Some(index), Some(500)).await {
                Ok(events) => {
                    // Get the length before moving the events
                    let num_events = events.len();
                    
                    // Extend events with the new data
                    self.events.extend(events);
                    let tot_events = self.events.len();

                    info!("Loaded {num_events} gamma events starting from index {index}");
                    
                    // Check if we have more events to load
                    if num_events < 500 || tot_events >= MAX_EVENTS {
                        info!("Reached end of gamma events pagination or max limit reached");
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

        // Finalize the markets list
        self.finalize_markets_loading();
        
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

    fn finalize_markets_loading(&mut self) {
        // Sort markets alphabetically by question
        self.markets.sort_by(|a, b| a.question.to_lowercase().cmp(&b.question.to_lowercase()));
        
        // prune empty events
        self.prune_empty_events();
        // Sort events alphabetically by title
        self.events.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        
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
                    .find(|m| m.tokens.iter().any(|t| t.token_id == token_id))
                    .map(|m| {
                        let token = m.tokens.iter().find(|t| t.token_id == token_id).unwrap();
                        format!("{} - {}", m.question, token.outcome)
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

                // Calculate market statistics with real API data
                let stats = super::stats::calculate_market_stats(&bids, &asks);

                // Preserve existing price history if updating the same token
                let price_history = if let Some(ref existing_orderbook) = self.orderbook {
                    if existing_orderbook.token_id == token_id {
                        // Keep existing price history and add new midpoint
                        let mut history = existing_orderbook.price_history.clone();
                        let mid_price = stats.mid_price;
                        if mid_price > 0.0 {
                            history.add_price(mid_price);
                        }
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
                    stats,
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
        super::price_history::update_bitcoin_price_if_needed(self);
        
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
}
