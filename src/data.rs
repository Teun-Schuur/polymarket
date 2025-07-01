use chrono::{DateTime, Utc};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct SimpleOrder {
    pub price: f64,
    pub size: f64,
    pub previous_size: f64,
    pub change_direction: OrderChangeDirection,
    pub change_timestamp: Option<Instant>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderChangeDirection {
    None,
    Increase,
    Decrease,
}

impl SimpleOrder {
    pub fn new(price: f64, size: f64) -> Self {
        Self {
            price,
            size,
            previous_size: size,
            change_direction: OrderChangeDirection::None,
            change_timestamp: None,
        }
    }
    
    pub fn update_size(&mut self, new_size: f64) {
        self.previous_size = self.size;
        
        if new_size > self.size {
            self.change_direction = OrderChangeDirection::Increase;
            self.change_timestamp = Some(Instant::now());
        } else if new_size < self.size {
            self.change_direction = OrderChangeDirection::Decrease;
            self.change_timestamp = Some(Instant::now());
        }
        
        self.size = new_size;
    }
    
    pub fn should_highlight(&self) -> bool {
        if let Some(timestamp) = self.change_timestamp {
            timestamp.elapsed().as_millis() < 1000 // Highlight for 1 second
        } else {
            false
        }
    }
    
    pub fn clear_highlight_if_expired(&mut self) {
        if let Some(timestamp) = self.change_timestamp {
            if timestamp.elapsed().as_millis() >= 1000 {
                self.change_direction = OrderChangeDirection::None;
                self.change_timestamp = None;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarketStats {
    pub total_volume: f64,
    pub volume_24h: f64,
    pub spread: f64,
    pub mid_price: f64,
    pub best_bid: f64,
    pub best_ask: f64,
    pub total_bid_size: f64,
    pub total_ask_size: f64,
}

impl Default for MarketStats {
    fn default() -> Self {
        Self {
            total_volume: 0.0,
            volume_24h: 0.0,
            spread: 0.0,
            mid_price: 0.0,
            best_bid: 0.0,
            best_ask: 0.0,
            total_bid_size: 0.0,
            total_ask_size: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderBookData {
    pub token_id: String,
    pub market_question: String,
    pub bids: Vec<SimpleOrder>,
    pub asks: Vec<SimpleOrder>,
    pub stats: MarketStats,
    pub tick_size: f64,
    pub last_updated: DateTime<Utc>,
    pub chart_center_price: Option<f64>,
    pub chart_needs_recentering: bool,
    pub price_history: PriceHistory,
}

#[derive(Debug, Clone)]
pub struct MarketInfo {
    pub question: String,
    pub tokens: Vec<TokenInfo>,
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token_id: String,
    pub outcome: String,
}

#[derive(Debug, Clone)]
pub struct PricePoint {
    pub timestamp: DateTime<Utc>,
    pub price: f64,
}

#[derive(Debug, Clone)]
pub struct PriceHistory {
    pub points: Vec<PricePoint>,
    pub max_points: usize,
}

impl PriceHistory {
    pub fn new(max_points: usize) -> Self {
        Self {
            points: Vec::with_capacity(max_points),
            max_points,
        }
    }
    
    pub fn add_price(&mut self, price: f64) {
        let now = Utc::now();
        
        // Only add if price is different from the last point (avoid duplicates)
        if let Some(last_point) = self.points.last() {
            if (last_point.price - price).abs() < f64::EPSILON {
                return;
            }
        }
        
        self.points.push(PricePoint {
            timestamp: now,
            price,
        });
        
        // Keep only the most recent points
        if self.points.len() > self.max_points {
            self.points.remove(0);
        }
    }
    
    pub fn get_price_range(&self) -> Option<(f64, f64)> {
        if self.points.is_empty() {
            return None;
        }
        
        let mut min_price = f64::INFINITY;
        let mut max_price = f64::NEG_INFINITY;
        
        for point in &self.points {
            min_price = min_price.min(point.price);
            max_price = max_price.max(point.price);
        }
        
        // Add some padding (5% on each side)
        let range = max_price - min_price;
        let padding = range * 0.05;
        
        Some((min_price - padding, max_price + padding))
    }
    
    pub fn get_time_range(&self) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
        if self.points.is_empty() {
            return None;
        }
        
        let min_time = self.points.first()?.timestamp;
        let max_time = self.points.last()?.timestamp;
        
        Some((min_time, max_time))
    }
    
    pub fn current_price(&self) -> Option<f64> {
        self.points.last().map(|p| p.price)
    }
}

#[derive(Debug, Clone)]
pub struct BitcoinPrice {
    pub price: f64,
    pub timestamp: DateTime<Utc>,
    pub history: PriceHistory,
}

impl Default for BitcoinPrice {
    fn default() -> Self {
        Self::new()
    }
}

impl BitcoinPrice {
    pub fn new() -> Self {
        Self {
            price: 0.0,
            timestamp: Utc::now(),
            history: PriceHistory::new(300), // Store last 300 points
        }
    }
    
    pub fn update_price(&mut self, new_price: f64) {
        self.price = new_price;
        self.timestamp = Utc::now();
        self.history.add_price(new_price);
    }
}
