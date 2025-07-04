#![allow(dead_code)]

use binance::websockets::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use cli_log::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CryptoSymbol {
    Bitcoin,
    Ethereum,
    Solana,
}

impl CryptoSymbol {
    pub fn ticker(&self) -> &'static str {
        match self {
            CryptoSymbol::Bitcoin => "btcusdt@bookTicker",
            CryptoSymbol::Ethereum => "ethusdt@bookTicker", 
            CryptoSymbol::Solana => "solusdt@bookTicker",
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            CryptoSymbol::Bitcoin => "Bitcoin",
            CryptoSymbol::Ethereum => "Ethereum",
            CryptoSymbol::Solana => "Solana",
        }
    }
    
    pub fn symbol(&self) -> &'static str {
        match self {
            CryptoSymbol::Bitcoin => "BTC",
            CryptoSymbol::Ethereum => "ETH",
            CryptoSymbol::Solana => "SOL",
        }
    }
}

pub struct CryptoWebSocket {
    pub thread_handles: HashMap<CryptoSymbol, thread::JoinHandle<()>>,
    pub keep_running: Arc<AtomicBool>,
    pub prices: Arc<Mutex<HashMap<CryptoSymbol, f64>>>,
}

impl Default for CryptoWebSocket {
    fn default() -> Self {
        Self::new()
    }
}

impl CryptoWebSocket {
    pub fn new() -> Self {
        Self {
            thread_handles: HashMap::new(),
            keep_running: Arc::new(AtomicBool::new(false)),
            prices: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start a single crypto symbol WebSocket connection
    pub fn start_single(&mut self, symbol: CryptoSymbol) {
        info!("Starting single crypto WebSocket connection for: {}", symbol.name());
        
        self.keep_running.store(true, Ordering::Relaxed);
        self.start_symbol(symbol);
    }
    
    /// Start multiple crypto symbol WebSocket connections (existing method)
    pub fn start(&mut self, symbols: Vec<CryptoSymbol>) {
        info!("Starting crypto WebSocket connections for: {symbols:?}");
        
        self.keep_running.store(true, Ordering::Relaxed);
        
        for symbol in symbols {
            self.start_symbol(symbol);
        }
    }
    
    fn start_symbol(&mut self, symbol: CryptoSymbol) {
        let keep_running = Arc::clone(&self.keep_running);
        let prices = Arc::clone(&self.prices);
        let ticker = symbol.ticker().to_string();
        let name = symbol.name();
        let symbol_clone = symbol.clone();

        let handle = thread::spawn(move || {
            let mut web_socket = WebSockets::new(move |event: WebsocketEvent| {
                match event {
                    WebsocketEvent::BookTicker(ticker_data) => {
                        // Parse the best bid and ask prices and average them
                        if let (Ok(bid), Ok(ask)) = (ticker_data.best_bid.parse::<f64>(), ticker_data.best_ask.parse::<f64>()) {
                            let mid_price = (bid + ask) / 2.0;
                            
                            if let Ok(mut price_map) = prices.lock() {
                                price_map.insert(symbol_clone.clone(), mid_price);
                            }
                        }
                    }
                    WebsocketEvent::DayTicker(ticker_data) => {
                       warn!("Received DayTicker event for {name}: {ticker_data:?}");
                    }
                    _ => {
                        warn!("Unexpected event received for {name}: {event:?}");
                        if let Ok(json) = serde_json::to_string(&event) {
                            warn!("Raw JSON event: {json}");
                        } else {
                            warn!("Failed to serialize event to JSON");
                        }
                    }
                }
                
                Ok(())
            });

            if let Err(e) = web_socket.connect(&ticker) {
                warn!("Failed to connect {name} WebSocket: {e}");
                return;
            }

            info!("{name} WebSocket connected successfully");

            if let Err(e) = web_socket.event_loop(&keep_running) {
                warn!("{name} WebSocket event loop error: {e}");
            }

            if let Err(e) = web_socket.disconnect() {
                warn!("Failed to disconnect {name} WebSocket: {e}");
            }

            info!("{name} WebSocket disconnected");
        });

        self.thread_handles.insert(symbol, handle);
    }

    pub fn stop(&mut self) {
        info!("Stopping crypto WebSockets...");
        self.keep_running.store(false, Ordering::Relaxed);
        
        for (symbol, handle) in self.thread_handles.drain() {
            if let Err(e) = handle.join() {
                warn!("Error joining {} WebSocket thread: {:?}", symbol.name(), e);
            }
        }
    }

    pub fn get_price(&self, symbol: &CryptoSymbol) -> f64 {
        if let Ok(price_map) = self.prices.lock() {
            *price_map.get(symbol).unwrap_or(&0.0)
        } else {
            0.0
        }
    }
    
    pub fn get_all_prices(&self) -> HashMap<CryptoSymbol, f64> {
        if let Ok(price_map) = self.prices.lock() {
            price_map.clone()
        } else {
            HashMap::new()
        }
    }

    pub fn is_running(&self) -> bool {
        self.keep_running.load(Ordering::Relaxed)
    }
}

impl Drop for CryptoWebSocket {
    fn drop(&mut self) {
        self.stop();
    }
}