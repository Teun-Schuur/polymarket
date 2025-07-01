#![allow(dead_code)]

use binance::websockets::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use serde_json::Value;
use cli_log::*;

pub struct BitcoinWebSocket {
    pub thread_handle: Option<thread::JoinHandle<()>>,
    pub keep_running: Arc<AtomicBool>,
    pub last_price: Arc<Mutex<f64>>,
}

impl Default for BitcoinWebSocket {
    fn default() -> Self {
        Self::new()
    }
}

impl BitcoinWebSocket {
    pub fn new() -> Self {
        Self {
            thread_handle: None,
            keep_running: Arc::new(AtomicBool::new(false)),
            last_price: Arc::new(Mutex::new(0.0)),
        }
    }

    pub fn start(&mut self) {
        info!("Starting Bitcoin WebSocket connection...");
        
        self.keep_running.store(true, Ordering::Relaxed);
        let keep_running = Arc::clone(&self.keep_running);
        let last_price = Arc::clone(&self.last_price);

        let handle = thread::spawn(move || {
            let btc_ticker = String::from("btcusdt@bookTicker");
            
            let mut web_socket = WebSockets::new(|event: WebsocketEvent| {
                match event {
                    WebsocketEvent::BookTicker(ticker) => {
                        // Parse the best bid and ask prices and average them
                        if let (Ok(bid), Ok(ask)) = (ticker.best_bid.parse::<f64>(), ticker.best_ask.parse::<f64>()) {
                            let mid_price = (bid + ask) / 2.0;
                            
                            if let Ok(mut price) = last_price.lock() {
                                *price = mid_price;
                            }
                            
                            // info!("Bitcoin price updated: ${mid_price:.2}");
                        }
                    }
                    WebsocketEvent::DayTicker(ticker) => {
                        // Fallback to day ticker if available
                        if let Ok(price) = ticker.current_close.parse::<f64>() {
                            if let Ok(mut last) = last_price.lock() {
                                *last = price;
                            }
                            
                            // info!("Bitcoin price updated (day ticker): ${:.2}", price);
                        }
                    }
                    _ => {
                        // Handle unexpected events or raw JSON
                        if let Ok(json_str) = serde_json::to_string(&event) {
                            if let Ok(json_value) = serde_json::from_str::<Value>(&json_str) {
                                // Try to extract price from different possible fields
                                if let Some(price) = json_value.get("c").and_then(|p| p.as_str()).and_then(|s| s.parse::<f64>().ok()) {
                                    if let Ok(mut last) = last_price.lock() {
                                        *last = price;
                                    }
                                } else if let Some(price) = json_value.get("b").and_then(|p| p.as_str()).and_then(|s| s.parse::<f64>().ok()) {
                                    if let Ok(mut last) = last_price.lock() {
                                        *last = price;
                                    }
                                }
                            }
                        }
                    }
                }
                
                Ok(())
            });

            if let Err(e) = web_socket.connect(&btc_ticker) {
                warn!("Failed to connect Bitcoin WebSocket: {e}");
                return;
            }

            info!("Bitcoin WebSocket connected successfully");

            if let Err(e) = web_socket.event_loop(&keep_running) {
                warn!("Bitcoin WebSocket event loop error: {e}");
            }

            if let Err(e) = web_socket.disconnect() {
                warn!("Failed to disconnect Bitcoin WebSocket: {e}");
            }

            info!("Bitcoin WebSocket disconnected");
        });

        self.thread_handle = Some(handle);
    }

    pub fn stop(&mut self) {
        info!("Stopping Bitcoin WebSocket...");
        self.keep_running.store(false, Ordering::Relaxed);
        
        if let Some(handle) = self.thread_handle.take() {
            if let Err(e) = handle.join() {
                warn!("Error joining Bitcoin WebSocket thread: {e:?}");
            }
        }
    }

    pub fn get_price(&self) -> f64 {
        if let Ok(price) = self.last_price.lock() {
            *price
        } else {
            0.0
        }
    }

    pub fn is_running(&self) -> bool {
        self.keep_running.load(Ordering::Relaxed)
    }
}

impl Drop for BitcoinWebSocket {
    fn drop(&mut self) {
        self.stop();
    }
}
