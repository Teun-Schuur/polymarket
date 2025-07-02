use anyhow::Result;
use polymarket_rs_client::ClobClient;
use std::{
    env,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use rust_decimal::prelude::*;
use cli_log::*;

use crate::data::{MarketInfo, MarketStats, OrderBookData, SimpleOrder, TokenInfo, PriceHistory};
use crate::websocket::{
    BookMessage, LastTradePriceMessage, PolymarketWebSocket, PolymarketWebSocketMessage,
    PriceChangeMessage, StructuredMessageCallback,
};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum SelectedTab {
    #[default]
    Orderbook,
    PriceHistory,
}

impl SelectedTab {
    /// Get the previous tab, if there is no previous tab return the current tab.
    pub fn previous(self) -> Self {
        match self {
            Self::Orderbook => Self::PriceHistory,
            Self::PriceHistory => Self::Orderbook,
        }
    }

    /// Get the next tab, if there is no next tab return the current tab.
    pub fn next(self) -> Self {
        match self {
            Self::Orderbook => Self::PriceHistory,
            Self::PriceHistory => Self::Orderbook,
        }
    }
}




pub struct App {
    pub client: ClobClient,
    pub orderbook: Option<OrderBookData>,
    pub markets: Vec<MarketInfo>,
    pub filtered_markets: Vec<usize>, // Indices into markets vec for filtering/sorting
    pub selected_market: usize,
    pub selected_token: usize,
    pub market_scroll_offset: usize,
    pub token_scroll_offset: usize,
    pub show_market_selector: bool,
    pub show_token_selector: bool,
    pub last_update: Instant,
    pub last_orderbook_update: Instant,
    pub update_interval: Duration,
    pub depth: usize,
    pub error_message: Option<String>,
    pub search_query: String,
    pub search_mode: bool,
    // UI state cache to avoid expensive calculations every frame
    pub needs_redraw: bool,
    // Tab system
    pub selected_tab: SelectedTab,
    // Price history data from API
    pub market_price_history: Option<polymarket_rs_client::PriceHistoryResponse>,
    // WebSocket integration for real-time updates
    pub current_websocket: Option<PolymarketWebSocket>,
    pub websocket_updates: Arc<Mutex<Vec<PolymarketWebSocketMessage>>>,
    pub last_websocket_check: Instant,
    // WebSocket reconnection attempts
    pub websocket_reconnect_attempts: u32,
    pub last_websocket_attempt: Instant,
    // Price history update timing
    pub last_price_history_update: Instant,
    pub price_history_update_interval: Duration,
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
        const POLYGON: u64 = 137;
        
        let mut client = ClobClient::with_l1_headers(HOST, &private_key, POLYGON);
        
        // Create or derive API key
        let nonce = None;
        let keys = client.create_or_derive_api_key(nonce).await.unwrap();
        
        client.set_api_creds(keys);
        
        Ok(Self {
            client,
            orderbook: None,
            markets: Vec::new(),
            filtered_markets: Vec::new(),
            selected_market: 0,
            selected_token: 0,
            market_scroll_offset: 0,
            token_scroll_offset: 0,
            show_market_selector: true,
            show_token_selector: false,
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
                    // Process the current batch of markets
                    if let Err(e) = self.process_markets_batch(&response).await {
                        warn!("Failed to process markets batch: {e:?}");
                        break;
                    }
                    
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
                                return self.process_markets_batch(&sampling_markets).await;
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
        
        // Finalize the markets list
        self.finalize_markets_loading();
        
        info!("Successfully loaded {} markets total", self.markets.len());
        Ok(())
    }

    async fn process_markets_batch(&mut self, response: &polymarket_rs_client::MarketsResponse) -> Result<()> {
        // Process the markets from the response data
        for market in &response.data {
            let token_infos: Vec<TokenInfo> = market.tokens.iter()
                .map(|token| TokenInfo {
                    token_id: token.token_id.clone(),
                    outcome: token.outcome.clone(),
                })
                .collect();
            
            if !token_infos.is_empty() {
                self.markets.push(MarketInfo {
                    question: market.question.clone(),
                    tokens: token_infos,
                });
            }
        }
        
        Ok(())
    }

    fn finalize_markets_loading(&mut self) {
        // Sort markets alphabetically by question
        self.markets.sort_by(|a, b| a.question.to_lowercase().cmp(&b.question.to_lowercase()));
        
        // Initialize filtered markets with all indices
        self.update_filtered_markets();
        
        // Reset selection if it's out of bounds
        if self.selected_market >= self.filtered_markets.len() {
            self.selected_market = 0;
            self.market_scroll_offset = 0;
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
                let stats = self.calculate_market_stats_with_api(&bids, &asks, token_id).await.unwrap_or_else(|_| {
                    Self::calculate_market_stats(&bids, &asks)
                });

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

        self.update_price_history_if_needed();
        self.update_bitcoin_price_if_needed();
        
        Ok(())
    }
    
    fn should_update_orderbook_via_api(&self) -> bool {
        if self.last_orderbook_update.elapsed() < self.update_interval {
            return false;
        }
        
        // If we have an active WebSocket, we don't need to poll the API
        self.current_websocket.is_none()
    }

    // Navigation methods
    pub fn select_market(&mut self) {
        if !self.filtered_markets.is_empty() && self.selected_market < self.filtered_markets.len() {
            self.selected_token = 0;
            self.token_scroll_offset = 0;
            self.show_market_selector = false;
            self.show_token_selector = true;
            self.search_mode = false;
            self.needs_redraw = true;
        }
    }

    pub fn select_token(&mut self) {
        if !self.filtered_markets.is_empty() && self.selected_market < self.filtered_markets.len() {
            let market_idx = self.filtered_markets[self.selected_market];
            let market = &self.markets[market_idx];
            if self.selected_token < market.tokens.len() {
                self.show_token_selector = false;
                self.needs_redraw = true;
                // We'll load the orderbook in the main loop
            }
        }
    }

    pub fn next_market(&mut self) {
        if !self.filtered_markets.is_empty() {
            self.selected_market = (self.selected_market + 1) % self.filtered_markets.len();
            self.needs_redraw = true;
        }
    }

    pub fn previous_market(&mut self) {
        if !self.filtered_markets.is_empty() {
            self.selected_market = if self.selected_market == 0 {
                self.filtered_markets.len() - 1
            } else {
                self.selected_market - 1
            };
            self.needs_redraw = true;
        }
    }

    pub fn page_down_markets(&mut self) {
        if !self.filtered_markets.is_empty() {
            let page_size = 10; // Adjust based on terminal height
            self.selected_market = std::cmp::min(
                self.selected_market + page_size,
                self.filtered_markets.len() - 1
            );
            self.needs_redraw = true;
        }
    }

    pub fn page_up_markets(&mut self) {
        let page_size = 10; // Adjust based on terminal height
        self.selected_market = self.selected_market.saturating_sub(page_size);
        self.needs_redraw = true;
    }

    pub fn next_token(&mut self) {
        if !self.filtered_markets.is_empty() && self.selected_market < self.filtered_markets.len() {
            let market_idx = self.filtered_markets[self.selected_market];
            let market = &self.markets[market_idx];
            if !market.tokens.is_empty() {
                self.selected_token = (self.selected_token + 1) % market.tokens.len();
                self.needs_redraw = true;
            }
        }
    }

    pub fn previous_token(&mut self) {
        if !self.filtered_markets.is_empty() && self.selected_market < self.filtered_markets.len() {
            let market_idx = self.filtered_markets[self.selected_market];
            let market = &self.markets[market_idx];
            if !market.tokens.is_empty() {
                self.selected_token = if self.selected_token == 0 {
                    market.tokens.len() - 1
                } else {
                    self.selected_token - 1
                };
                self.needs_redraw = true;
            }
        }
    }

    pub fn page_down_tokens(&mut self) {
        if !self.filtered_markets.is_empty() && self.selected_market < self.filtered_markets.len() {
            let market_idx = self.filtered_markets[self.selected_market];
            let market = &self.markets[market_idx];
            if !market.tokens.is_empty() {
                let page_size = 10; // Adjust based on terminal height
                self.selected_token = std::cmp::min(
                    self.selected_token + page_size,
                    market.tokens.len() - 1
                );
                self.needs_redraw = true;
            }
        }
    }

    pub fn page_up_tokens(&mut self) {
        let page_size = 10; // Adjust based on terminal height
        self.selected_token = self.selected_token.saturating_sub(page_size);
        self.needs_redraw = true;
    }

    pub fn get_current_token_id(&self) -> Option<String> {
        if !self.filtered_markets.is_empty() && self.selected_market < self.filtered_markets.len() {
            let market_idx = self.filtered_markets[self.selected_market];
            let market = &self.markets[market_idx];
            if self.selected_token < market.tokens.len() {
                return Some(market.tokens[self.selected_token].token_id.clone());
            }
        }
        None
    }

    // Search functionality
    pub fn add_search_char(&mut self, ch: char) {
        self.search_query.push(ch);
        self.update_filtered_markets();
        self.selected_market = 0;
        self.market_scroll_offset = 0;
        self.needs_redraw = true;
    }

    pub fn remove_search_char(&mut self) {
        self.search_query.pop();
        self.update_filtered_markets();
        self.selected_market = 0;
        self.market_scroll_offset = 0;
        self.needs_redraw = true;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.update_filtered_markets();
        self.selected_market = 0;
        self.market_scroll_offset = 0;
        self.needs_redraw = true;
    }

    pub fn toggle_search_mode(&mut self) {
        self.search_mode = !self.search_mode;
        if !self.search_mode {
            self.clear_search();
        }
        self.needs_redraw = true;
    }

    // Market statistics calculations
    pub fn calculate_market_stats(bids: &[SimpleOrder], asks: &[SimpleOrder]) -> MarketStats {
        let best_bid = bids.first().map(|b| b.price).unwrap_or(0.0);
        let best_ask = asks.first().map(|a| a.price).unwrap_or(0.0);
        
        let mid_price = Self::calculate_weighted_midpoint(bids, asks);
        
        let spread = if best_bid > 0.0 && best_ask > 0.0 {
            best_ask - best_bid
        } else {
            0.0
        };
        
        let total_bid_size = bids.iter().map(|b| b.size).sum();
        let total_ask_size = asks.iter().map(|a| a.size).sum();
        
        MarketStats {
            total_volume: 0.0,
            volume_24h: 0.0,
            spread,
            mid_price,
            best_bid,
            best_ask,
            total_bid_size,
            total_ask_size,
        }
    }

    async fn calculate_market_stats_with_api(&self, bids: &[SimpleOrder], asks: &[SimpleOrder], token_id: &str) -> Result<MarketStats> {
        // Get basic stats from order book
        let best_bid = bids.first().map(|b| b.price).unwrap_or(0.0);
        let best_ask = asks.first().map(|a| a.price).unwrap_or(0.0);
        
        let mid_price = Self::calculate_weighted_midpoint(bids, asks);
        
        let spread = if best_bid > 0.0 && best_ask > 0.0 {
            best_ask - best_bid
        } else {
            0.0
        };
        
        let total_bid_size = bids.iter().map(|b| b.size).sum();
        let total_ask_size = asks.iter().map(|a| a.size).sum();
        
        // Fetch additional data from API
        let (volume_24h, total_volume) = match self.fetch_market_volume_data(token_id).await {
            Ok((v24h, total_vol)) => (v24h, total_vol),
            Err(_) => (0.0, 0.0), // Fallback to 0 if API call fails
        };
        
        Ok(MarketStats {
            total_volume,
            volume_24h,
            spread,
            mid_price,
            best_bid,
            best_ask,
            total_bid_size,
            total_ask_size,
        })
    }

    async fn fetch_market_volume_data(&self, token_id: &str) -> Result<(f64, f64)> {
        // Try to get market data for this token
        let condition_id = self.get_condition_id_for_token(token_id).await?;
        
        // Get market trades events which might contain volume data
        match self.client.get_market_trades_events(&condition_id).await {
            Ok(trades_data) => {
                // Parse volume data from trades
                let volume_24h = self.parse_24h_volume_from_trades(&trades_data);
                let total_volume = self.parse_total_volume_from_trades(&trades_data);
                Ok((volume_24h, total_volume))
            }
            Err(_) => {
                // Fallback: try to get last trade prices which might give us some volume info
                match self.client.get_last_trade_price(token_id).await {
                    Ok(_trade_data) => {
                        // For now, return 0 until we can parse the actual volume data
                        Ok((0.0, 0.0))
                    }
                    Err(_) => Ok((0.0, 0.0))
                }
            }
        }
    }

    async fn get_condition_id_for_token(&self, token_id: &str) -> Result<String> {
        // Look through our loaded markets to find the condition_id for this token
        for market in &self.markets {
            for token in &market.tokens {
                if token.token_id == token_id {
                    // For now, we'll use the market question as a fallback
                    // In a real implementation, you'd need to extract the actual condition_id
                    // from the market data structure
                    return Ok(market.question.clone());
                }
            }
        }
        Err(anyhow::anyhow!("Token not found in loaded markets"))
    }

    fn parse_24h_volume_from_trades(&self, trades_data: &serde_json::Value) -> f64 {
        // Parse 24h volume from trades data
        if let Some(volume) = trades_data.get("volume_24h").and_then(|v| v.as_f64()) {
            volume
        } else if let Some(events) = trades_data.get("events").and_then(|e| e.as_array()) {
            // Calculate volume from events in the last 24 hours
            let now = chrono::Utc::now();
            let twenty_four_hours_ago = now - chrono::Duration::hours(24);
            
            events.iter()
                .filter_map(|event| {
                    let timestamp = event.get("timestamp")?.as_str()?;
                    let parsed_time = chrono::DateTime::parse_from_rfc3339(timestamp).ok()?;
                    if parsed_time > twenty_four_hours_ago {
                        event.get("volume")?.as_f64()
                    } else {
                        None
                    }
                })
                .sum()
        } else {
            0.0
        }
    }

    fn parse_total_volume_from_trades(&self, trades_data: &serde_json::Value) -> f64 {
        // Parse total volume from trades data
        if let Some(volume) = trades_data.get("total_volume").and_then(|v| v.as_f64()) {
            volume
        } else if let Some(events) = trades_data.get("events").and_then(|e| e.as_array()) {
            // Calculate total volume from all events
            events.iter()
                .filter_map(|event| event.get("volume")?.as_f64())
                .sum()
        } else {
            0.0
        }
    }

    async fn get_tick_size_for_token(&self, token_id: &str) -> f64 {
        match self.client.get_tick_size(token_id).await {
            Ok(tick_size) => tick_size.to_f64().unwrap_or(0.0001),
            Err(_) => 0.0001, // Default tick size for prediction markets
        }
    }

    // WebSocket integration methods
    pub fn start_websocket_for_token(&mut self, token_id: &str) {
        info!("Starting WebSocket connection for token: {token_id}");
        
        // Close existing WebSocket if any
        if self.current_websocket.is_some() {
            info!("Closing existing WebSocket connection");
            self.current_websocket = None;
        }
        
        let updates_arc: Arc<Mutex<Vec<PolymarketWebSocketMessage>>> = Arc::clone(&self.websocket_updates);
        let token_id_owned = token_id.to_string();
        
        let callback: StructuredMessageCallback = Box::new(move |msg| {
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
        
        self.current_websocket = Some(PolymarketWebSocket::connect_structured(
            "market".into(),
            None,
            vec![token_id.to_string()],
            callback,
        ));
        
        info!("WebSocket started for token: {token_id}");
    }

    fn process_websocket_updates(&mut self) -> Result<()> {
        if self.last_websocket_check.elapsed() < Duration::from_millis(50) {
            return Ok(());
        }
        self.last_websocket_check = Instant::now();
        
        // Check WebSocket connection health
        self.check_websocket_health();
        
        // Process pending updates
        let updates = match self.websocket_updates.try_lock() {
            Ok(mut guard) if !guard.is_empty() => std::mem::take(&mut *guard),
            _ => return Ok(()),
        };
        
        for update in updates {
            self.apply_websocket_update(update)?;
        }
        
        Ok(())
    }
    
    fn check_websocket_health(&mut self) {
        if let Some(ref ws) = self.current_websocket {
            if ws.thread_handle.is_finished() {
                warn!("WebSocket thread terminated, reconnecting");
                self.current_websocket = None;
                if let Some(ref orderbook) = self.orderbook {
                    self.try_reconnect_websocket(&orderbook.token_id.clone());
                }
                self.needs_redraw = true;
            }
        }
    }
    
    fn apply_websocket_update(&mut self, update: PolymarketWebSocketMessage) -> Result<()> {
        let orderbook = match &mut self.orderbook {
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
                Self::apply_book_update_static(orderbook, &book_msg, self.depth)?;
            }
            PolymarketWebSocketMessage::PriceChange(price_msg) => {
                Self::apply_price_changes_static(orderbook, &price_msg, self.depth)?;
            }
            PolymarketWebSocketMessage::LastTradePrice(trade_msg) => {
                Self::apply_trade_update_static(orderbook, &trade_msg)?;
            }
            PolymarketWebSocketMessage::TickSizeChange(tick_msg) => {
                if let Ok(new_tick_size) = tick_msg.new_tick_size.parse::<f64>() {
                    orderbook.tick_size = new_tick_size;
                }
            }
            PolymarketWebSocketMessage::Unknown(_) => return Ok(()),
        }
        
        self.needs_redraw = true;
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
        Self::update_orderbook_stats_and_history(orderbook);
        
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
        Self::update_orderbook_stats_and_history(orderbook);
        
        Ok(())
    }

    fn apply_trade_update_static(orderbook: &mut OrderBookData, _trade_msg: &LastTradePriceMessage) -> Result<()> {
        orderbook.last_updated = chrono::Utc::now();
        Self::update_orderbook_stats_and_history(orderbook);
        Ok(())
    }

    fn update_orderbook_stats_and_history(orderbook: &mut OrderBookData) {
        orderbook.stats = App::calculate_market_stats(&orderbook.bids, &orderbook.asks);
        
        // Add current midpoint to price history
        let mid_price = orderbook.stats.mid_price;
        if mid_price > 0.0 {
            orderbook.price_history.add_price(mid_price);
        }
    }

    pub fn try_reconnect_websocket(&mut self, token_id: &str) {
        const MAX_ATTEMPTS: u32 = 5;
        const DELAY: Duration = Duration::from_secs(10);
        
        if self.websocket_reconnect_attempts >= MAX_ATTEMPTS 
            || self.last_websocket_attempt.elapsed() < DELAY {
            return;
        }
        
        info!("Reconnecting WebSocket (attempt {}/{})", 
              self.websocket_reconnect_attempts + 1, MAX_ATTEMPTS);
        
        self.websocket_reconnect_attempts += 1;
        self.last_websocket_attempt = Instant::now();
        self.start_websocket_for_token(token_id);
    }
    
    pub fn reset_websocket_reconnect_counter(&mut self) {
        self.websocket_reconnect_attempts = 0;
    }

    // Remove these overly complex methods that add unnecessary complexity
    // The main loop will handle UI updates based on the simple timer mechanism

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

    pub fn should_update_price_history(&self) -> bool {
        self.last_price_history_update.elapsed() >= self.price_history_update_interval
    }

    pub fn update_price_history_if_needed(&mut self) {
        if self.should_update_price_history() {
            if let Some(ref mut orderbook) = self.orderbook {
                let weighted_mid_price = Self::calculate_weighted_midpoint(&orderbook.bids, &orderbook.asks);
                if weighted_mid_price > 0.0 {
                    orderbook.price_history.add_price(weighted_mid_price);
                    self.last_price_history_update = Instant::now();
                }
            }
        }
    }

    pub fn should_show_bitcoin_chart(&self) -> bool {
        if let Some(ref orderbook) = self.orderbook {
            orderbook.market_question.to_lowercase().contains("bitcoin") || 
            orderbook.market_question.to_lowercase().contains("btc")
        } else {
            false
        }
    }

    pub fn start_bitcoin_websocket_if_needed(&mut self) {
        if self.should_show_bitcoin_chart() && !self.bitcoin_websocket_active {
            info!("Starting Bitcoin WebSocket - market contains 'Bitcoin'");
            
            // Initialize Bitcoin price tracking with Arc<Mutex<>>
            self.bitcoin_price = Some(Arc::new(Mutex::new(crate::data::BitcoinPrice::new())));
            
            // Start the WebSocket in a separate thread
            self.start_bitcoin_websocket();
            self.bitcoin_websocket_active = true;
        } else if !self.should_show_bitcoin_chart() && self.bitcoin_websocket_active {
            info!("Stopping Bitcoin WebSocket - market no longer contains 'Bitcoin'");
            self.stop_bitcoin_websocket();
        }
    }

    fn start_bitcoin_websocket(&mut self) {
        use crate::websocket::BitcoinWebSocket;
        use std::thread;
        
        if let Some(ref bitcoin_price_arc) = self.bitcoin_price {
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

    fn stop_bitcoin_websocket(&mut self) {
        self.bitcoin_websocket_active = false;
        self.bitcoin_price = None;
    }

    pub fn update_bitcoin_price_if_needed(&mut self) {
        // This will be called periodically to check if we need to start/stop Bitcoin tracking
        self.start_bitcoin_websocket_if_needed();
    }

    // Tab navigation methods
    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
        self.needs_redraw = true;
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
        self.needs_redraw = true;
    }
}
