//! Market statistics calculations

use anyhow::Result;
use crate::data::{MarketStats, SimpleOrder};
use super::core::App;

pub fn calculate_market_stats(bids: &[SimpleOrder], asks: &[SimpleOrder]) -> MarketStats {
    let best_bid = bids.first().map(|b| b.price).unwrap_or(0.0);
    let best_ask = asks.first().map(|a| a.price).unwrap_or(0.0);
    
    let mid_price = calculate_weighted_midpoint(bids, asks);
    
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

pub fn calculate_weighted_midpoint(bids: &[SimpleOrder], asks: &[SimpleOrder]) -> f64 {
    if bids.is_empty() || asks.is_empty() {
        return 0.0;
    }

    let best_bid = bids[0].price;
    let best_ask = asks[0].price;
    let bid_size = bids[0].size;
    let ask_size = asks[0].size;

    if bid_size + ask_size == 0.0 {
        return (best_bid + best_ask) / 2.0;
    }

    // Weight the midpoint by the sizes at the best levels
    (best_bid * ask_size + best_ask * bid_size) / (bid_size + ask_size)
}

impl App {
    pub async fn calculate_market_stats_with_api(&self, bids: &[SimpleOrder], asks: &[SimpleOrder], token_id: &str) -> Result<MarketStats> {
        // Get basic stats from order book
        let best_bid = bids.first().map(|b| b.price).unwrap_or(0.0);
        let best_ask = asks.first().map(|a| a.price).unwrap_or(0.0);
        
        let mid_price = calculate_weighted_midpoint(bids, asks);
        
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
}
