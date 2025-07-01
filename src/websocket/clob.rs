use cli_log::{
    info,
    warn,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::Message,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use serde_json::json;
use serde::{Deserialize, Serialize};

// Structured data types for WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSummary {
    pub price: String,
    pub size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookMessage {
    pub event_type: String,
    pub asset_id: String,
    pub market: String,
    pub timestamp: String,
    pub hash: String,
    pub bids: Vec<OrderSummary>,
    pub asks: Vec<OrderSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceChange {
    pub price: String,
    pub side: String,
    pub size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceChangeMessage {
    pub event_type: String,
    pub asset_id: String,
    pub market: String,
    pub timestamp: String,
    pub hash: String,
    pub changes: Vec<PriceChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickSizeChangeMessage {
    pub event_type: String,
    pub asset_id: String,
    pub market: String,
    pub old_tick_size: String,
    pub new_tick_size: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastTradePriceMessage {
    pub event_type: String,
    pub asset_id: String,
    pub market: String,
    pub price: String,
    pub side: String,
    pub size: String,
    pub fee_rate_bps: String,
    pub timestamp: String,
}

#[derive(Debug, Clone)]
pub enum PolymarketWebSocketMessage {
    Book(BookMessage),
    PriceChange(PriceChangeMessage),
    TickSizeChange(TickSizeChangeMessage),
    LastTradePrice(LastTradePriceMessage),
    Unknown(String),
}

// Callback type for handling structured messages
pub type StructuredMessageCallback = Box<dyn Fn(PolymarketWebSocketMessage) + Send>;
// Legacy callback for raw messages
pub type MessageCallback = Box<dyn Fn(Message) + Send>;

pub struct PolymarketWebSocket {
    sender: Sender<Message>,
    pub thread_handle: thread::JoinHandle<()>,
}

impl PolymarketWebSocket {
    pub fn connect(
        channel_type: String,
        auth: Option<serde_json::Value>,
        filter_ids: Vec<String>,
        callback: MessageCallback,
    ) -> Self {
        let (tx, _rx) = channel();
        let channel = channel_type.clone();
        let filter: Vec<String> = filter_ids;

        let handle = thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let url = format!("wss://ws-subscriptions-clob.polymarket.com/ws/{channel}");
                info!("Connecting to WebSocket at: {url}");
                
                let (ws_stream, _) = match connect_async(&url).await {
                    Ok(stream) => {
                        info!("✅ Successfully connected to WebSocket at: {url}");
                        stream
                    },
                    Err(e) => {
                        warn!("❌ Failed to connect to WebSocket: {e}");
                        warn!("Make sure you have TLS support enabled. Try: cargo add tokio-tungstenite --features native-tls");
                        return;
                    }
                };
                
                let (mut write, mut read) = ws_stream.split();

                // Send subscription message
                let sub_msg = match channel_type.as_str() {
                    "user" => json!({
                        "type": "user",
                        "markets": filter,
                        "auth": auth.as_ref().map(|auth| {
                            json!({
                                "apiKey": auth["apiKey"],
                                "secret": auth["secret"],
                                "passphrase": auth["passphrase"]
                            })
                        }).expect("Auth is required for user channel")
                    }),
                    "market" => json!({
                        "type": "market",
                        "assets_ids": filter
                    }),
                    _ => panic!("Invalid channel type"),
                };
                
                if let Err(e) = write.send(Message::Text(sub_msg.to_string().into())).await {
                    warn!("Failed to send subscription message: {e:?}");
                    return;
                }

                // Message processing loop with improved error handling
                while let Some(msg_result) = read.next().await {
                    match msg_result {
                        Ok(msg) => {
                            match &msg {
                                Message::Text(_) | Message::Binary(_) => {
                                    callback(msg.clone());
                                }
                                Message::Ping(data) => {
                                    if let Err(e) = write.send(Message::Pong(data.clone())).await {
                                        warn!("Failed to send pong: {e}");
                                        break;
                                    }
                                }
                                Message::Pong(_) => {
                                    // Pong received, connection is alive
                                }
                                Message::Close(close_frame) => {
                                    if let Some(frame) = close_frame {
                                        warn!("WebSocket closed with code: {} reason: {}", frame.code, frame.reason);
                                    } else {
                                        warn!("WebSocket closed without close frame");
                                    }
                                    break;
                                }
                                Message::Frame(_) => {
                                    // Raw frame, usually handled internally
                                }
                            }
                        }
                        Err(e) => {
                            match e {
                                tokio_tungstenite::tungstenite::Error::Protocol(protocol_error) => {
                                    warn!("WebSocket protocol error: {protocol_error}");
                                    // Try to reconnect on protocol errors
                                    warn!("Connection lost, attempting to reconnect...");
                                    break;
                                }
                                tokio_tungstenite::tungstenite::Error::ConnectionClosed => {
                                    warn!("WebSocket connection closed by remote");
                                    break;
                                }
                                _ => {
                                    warn!("WebSocket error: {e}");
                                    break;
                                }
                            }
                        }
                    }
                }
                
                // Attempt graceful closure
                if let Err(e) = write.send(Message::Close(None)).await {
                    warn!("Failed to send close message: {e}");
                }
                
                warn!("WebSocket connection ended for channel: {channel}");
            });
        });

        Self {
            sender: tx,
            thread_handle: handle,
        }
    }

    pub fn send(&self, message: Message) {
        self.sender.send(message).unwrap();
    }

    /// Connect with structured message parsing
    pub fn connect_structured(
        channel_type: String,
        auth: Option<serde_json::Value>,
        filter_ids: Vec<String>,
        callback: StructuredMessageCallback,
    ) -> Self {
        let (tx, _rx) = channel();
        let channel = channel_type.clone();
        let filter: Vec<String> = filter_ids;

        let handle = thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let url = format!("wss://ws-subscriptions-clob.polymarket.com/ws/{channel}");
                info!("Connecting to WebSocket at: {url}");
                
                let (ws_stream, _) = match connect_async(&url).await {
                    Ok(stream) => stream,
                    Err(e) => {
                        warn!("Failed to connect to WebSocket: {e:?}");
                        warn!("Make sure you have TLS support enabled. Try: cargo add tokio-tungstenite --features native-tls");
                        return;
                    }
                };
                
                let (mut write, mut read) = ws_stream.split();

                // Send subscription message
                let sub_msg = match channel_type.as_str() {
                    "user" => json!({
                        "type": "user",
                        "markets": filter,
                        "auth": auth.as_ref().map(|auth| {
                            json!({
                                "apiKey": auth["apiKey"],
                                "secret": auth["secret"],
                                "passphrase": auth["passphrase"]
                            })
                        }).expect("Auth is required for user channel")
                    }),
                    "market" => json!({
                        "type": "market",
                        "assets_ids": filter
                    }),
                    _ => panic!("Invalid channel type"),
                };
                
                if let Err(e) = write.send(Message::Text(sub_msg.to_string().into())).await {
                    warn!("Failed to send subscription message: {e:?}");
                    return;
                }

                info!("✅ Connected and subscribed to {channel_type} channel");

                // Message processing loop
                while let Some(msg_result) = read.next().await {
                    match msg_result {
                        Ok(msg) => {
                            // Parse and handle structured messages
                            Self::handle_structured_message(&msg, &callback);
                            
                            // Respond to pings
                            if let Message::Ping(data) = msg {
                                if let Err(e) = write.send(Message::Pong(data)).await {
                                    warn!("Failed to send pong: {e:?}");
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            warn!("WebSocket error: {e:?}");
                            break;
                        }
                    }
                }
            });
        });

        Self {
            sender: tx,
            thread_handle: handle,
        }
    }

    fn handle_structured_message(msg: &Message, callback: &StructuredMessageCallback) {
        if let Message::Text(text) = msg {
            let text_str = text.to_string();
            // Parse the JSON array (messages come as arrays)
            match serde_json::from_str::<Vec<serde_json::Value>>(&text_str) {
                Ok(messages) => {
                    for message_value in messages {
                        let parsed_msg = Self::parse_polymarket_message(message_value);
                        callback(parsed_msg);
                    }
                }
                Err(_e) => {
                    // Try parsing as a single object
                    match serde_json::from_str::<serde_json::Value>(&text_str) {
                        Ok(message_value) => {
                            let parsed_msg = Self::parse_polymarket_message(message_value);
                            callback(parsed_msg);
                        }
                        Err(_e2) => {
                            warn!("Failed to parse JSON: {text_str}");
                            callback(PolymarketWebSocketMessage::Unknown(text_str));
                        }
                    }
                }
            }
        }
    }

    fn parse_polymarket_message(value: serde_json::Value) -> PolymarketWebSocketMessage {
        let event_type = value.get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let value_string = value.to_string(); // Create this once for error cases

        match event_type {
            "book" => {
                match serde_json::from_value::<BookMessage>(value) {
                    Ok(book_msg) => {
                        info!("📊 Book Update: {} bids, {} asks for asset {}", 
                                book_msg.bids.len(), 
                                book_msg.asks.len(), 
                                &book_msg.asset_id[..10]);
                        PolymarketWebSocketMessage::Book(book_msg)
                    }
                    Err(e) => {
                        warn!("Failed to parse book message: {e:?}");
                        PolymarketWebSocketMessage::Unknown(value_string)
                    }
                }
            }
            "price_change" => {
                match serde_json::from_value::<PriceChangeMessage>(value) {
                    Ok(price_msg) => {
                        // Debug log the changes to see what side they're on
                        // for change in &price_msg.changes {
                            // info!("🔍 WS Price Change: side={}, price={}, size={}", 
                                //   change.side, change.price, change.size);
                        // }
                        // info!("💱 Price Change: {} changes for asset {}", 
                        //         price_msg.changes.len(), 
                        //         &price_msg.asset_id[..10]);
                        PolymarketWebSocketMessage::PriceChange(price_msg)
                    }
                    Err(e) => {
                        warn!("Failed to parse price change message: {e:?}");
                        PolymarketWebSocketMessage::Unknown(value_string)
                    }
                }
            }
            "tick_size_change" => {
                match serde_json::from_value::<TickSizeChangeMessage>(value) {
                    Ok(tick_msg) => {
                        info!("📏 Tick Size Change: {} -> {} for asset {}", 
                                tick_msg.old_tick_size, 
                                tick_msg.new_tick_size, 
                                &tick_msg.asset_id[..10]);
                        PolymarketWebSocketMessage::TickSizeChange(tick_msg)
                    }
                    Err(e) => {
                        warn!("Failed to parse tick size change message: {e:?}");
                        PolymarketWebSocketMessage::Unknown(value_string)
                    }
                }
            }
            "last_trade_price" => {
                match serde_json::from_value::<LastTradePriceMessage>(value) {
                    Ok(trade_msg) => {
                        info!("💰 Trade: {} {} @ {} for asset {}", 
                                trade_msg.side, 
                                trade_msg.size, 
                                trade_msg.price, 
                                &trade_msg.asset_id[..10]);
                        PolymarketWebSocketMessage::LastTradePrice(trade_msg)
                    }
                    Err(e) => {
                        warn!("Failed to parse last trade price message: {e:?}");
                        PolymarketWebSocketMessage::Unknown(value_string)
                    }
                }
            }
            _ => {
                info!("❓ Unknown message type: {event_type}");
                PolymarketWebSocketMessage::Unknown(value_string)
            }
        }
    }
}
