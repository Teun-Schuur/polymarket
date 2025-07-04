use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Order {
    pub price: f64,
    pub size: f64,
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    pub token_id: String,
    pub market_question: String,
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
    pub tick_size: f64,
    pub last_updated: DateTime<Utc>,
}

impl OrderBook {
    pub fn get_spread(&self) -> f64 {
        if let (Some(best_bid), Some(best_ask)) = (self.bids.first(), self.asks.first()) {
            best_ask.price - best_bid.price
        } else {
            0.0
        }
    }

    pub fn get_midpoint(&self) -> f64 {
        if let (Some(best_bid), Some(best_ask)) = (self.bids.first(), self.asks.first()) {
            (best_bid.price + best_ask.price) / 2.0
        } else {
            0.0
        }
    }
}


#[derive(Debug, Clone)]
pub struct OrderBooks {
    pub books: HashMap<String, (OrderBook, bool)>,
}

impl OrderBooks {
    pub fn new() -> Self {
        OrderBooks {
            books: HashMap::new(),
        }
    }

    pub fn add_orderbook(&mut self, orderbook: OrderBook) {
        self.books.insert(orderbook.token_id.clone(), (orderbook, true));
    }

    pub fn get_orderbook(&self, token_id: &str) -> Option<&OrderBook> {
        self.books.get(token_id).map(|(orderbook, _)| orderbook)
    }
}
