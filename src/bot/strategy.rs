use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::data::OrderBookData;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StrategyType {
    ArbitrageDetector,
    PriceAnomaly,
    VolumeSpike,
    CrossMarketCorrelation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyScope {
    SingleMarket,  // Operates on individual markets
    Event,         // Operates on events (and their sub-markets)
    MultiMarket,   // Operates on multiple individual markets
}

impl StrategyType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::ArbitrageDetector => "Arbitrage Detector",
            Self::PriceAnomaly => "Price Anomaly Scanner",
            Self::VolumeSpike => "Volume Spike Alert",
            Self::CrossMarketCorrelation => "Cross-Market Correlation",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::ArbitrageDetector => "Finds price discrepancies between markets in the same event",
            Self::PriceAnomaly => "Detects unusual price movements in individual markets",
            Self::VolumeSpike => "Alerts on sudden volume increases in individual markets",
            Self::CrossMarketCorrelation => "Analyzes correlation between multiple markets",
        }
    }

    pub fn scope(&self) -> StrategyScope {
        match self {
            Self::ArbitrageDetector => StrategyScope::Event,
            Self::PriceAnomaly => StrategyScope::SingleMarket,
            Self::VolumeSpike => StrategyScope::SingleMarket,
            Self::CrossMarketCorrelation => StrategyScope::MultiMarket,
        }
    }

    pub fn requires_multiple_markets(&self) -> bool {
        matches!(self, Self::ArbitrageDetector | Self::CrossMarketCorrelation)
    }

    pub fn all_strategies() -> Vec<Self> {
        vec![
            Self::ArbitrageDetector,
            Self::PriceAnomaly,
            Self::VolumeSpike,
            Self::CrossMarketCorrelation,
        ]
    }
}

#[derive(Debug, Clone)]
pub enum StrategyStatus {
    Stopped,
    Running,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyAlert {
    pub timestamp: DateTime<Utc>,
    pub strategy: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub market_data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl AlertSeverity {
    pub fn color(&self) -> ratatui::style::Color {
        match self {
            Self::Low => ratatui::style::Color::Green,
            Self::Medium => ratatui::style::Color::Yellow,
            Self::High => ratatui::style::Color::LightRed,
            Self::Critical => ratatui::style::Color::Red,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Strategy {
    pub strategy_type: StrategyType,
    pub status: StrategyStatus,
    pub selected_market_ids: Vec<String>, 
    pub selected_market_names: Vec<String>,
    pub selected_event_ids: Vec<String>,
    pub selected_event_names: Vec<String>,
    pub alerts: Vec<StrategyAlert>,
    pub last_run: Option<DateTime<Utc>>,
    pub run_count: u64,
    // For tracking multiple orderbooks (used by arbitrage detector)
    pub orderbooks: HashMap<String, OrderBookData>,
    pub last_arbitrage_check: Option<DateTime<Utc>>,
}

impl Strategy {
    pub fn new(strategy_type: StrategyType) -> Self {
        Self {
            strategy_type,
            status: StrategyStatus::Stopped,
            selected_market_ids: Vec::new(),
            selected_market_names: Vec::new(),
            selected_event_ids: Vec::new(),
            selected_event_names: Vec::new(),
            alerts: Vec::new(),
            last_run: None,
            run_count: 0,
            orderbooks: HashMap::new(),
            last_arbitrage_check: None,
        }
    }

    pub fn get_selection_summary(&self) -> String {
        match self.strategy_type.scope() {
            StrategyScope::Event => {
                if self.selected_event_ids.is_empty() {
                    "No events selected".to_string()
                } else {
                    format!("{} event(s) selected", self.selected_event_ids.len())
                }
            }
            StrategyScope::SingleMarket | StrategyScope::MultiMarket => {
                if self.selected_market_ids.is_empty() {
                    "No markets selected".to_string()
                } else {
                    format!("{} market(s) selected", self.selected_market_ids.len())
                }
            }
        }
    }

    pub fn update_orderbook(&mut self, orderbook: OrderBookData) {
        self.orderbooks.insert(orderbook.token_id.clone(), orderbook);
    }

    pub fn check_arbitrage_opportunities(&mut self) -> Vec<StrategyAlert> {
        if self.strategy_type != StrategyType::ArbitrageDetector {
            return Vec::new();
        }

        let mut alerts = Vec::new();
        let now = Utc::now();

        // For each event, check if we have orderbooks for all markets
        for event_id in &self.selected_event_ids {
            let event_markets = self.get_event_markets(event_id);
            if event_markets.len() < 2 {
                continue; // Need at least 2 markets for arbitrage
            }

            // Check if the sum of all "Yes" prices is < 1.0
            let yes_prices: Vec<f64> = event_markets
                .iter()
                .filter_map(|token_id| {
                    self.orderbooks.get(token_id).and_then(|ob| {
                        ob.bids.first().map(|bid| bid.price)
                    })
                })
                .collect();

            if yes_prices.len() == event_markets.len() {
                let total_yes_price: f64 = yes_prices.iter().sum();
                if total_yes_price < 1.0 {
                    let arbitrage_amount = 1.0 - total_yes_price;
                    let alert = StrategyAlert {
                        timestamp: now,
                        strategy: "Arbitrage Detector".to_string(),
                        severity: if arbitrage_amount > 0.1 { AlertSeverity::High } else { AlertSeverity::Medium },
                        message: format!(
                            "Arbitrage opportunity detected! Sum of YES prices: {total_yes_price:.4} (opportunity: ${arbitrage_amount:.4})"
                        ),
                        market_data: HashMap::new(),
                    };
                    alerts.push(alert);
                }
            }

            // Check if the sum of all "No" prices is < 1.0
            let no_prices: Vec<f64> = event_markets
                .iter()
                .filter_map(|token_id| {
                    self.orderbooks.get(token_id).and_then(|ob| {
                        ob.asks.first().map(|ask| ask.price)
                    })
                })
                .collect();

            if no_prices.len() == event_markets.len() {
                let total_no_price: f64 = no_prices.iter().sum();
                if total_no_price < 1.0 {
                    let arbitrage_amount = 1.0 - total_no_price;
                    let alert = StrategyAlert {
                        timestamp: now,
                        strategy: "Arbitrage Detector".to_string(),
                        severity: if arbitrage_amount > 0.1 { AlertSeverity::High } else { AlertSeverity::Medium },
                        message: format!(
                            "Arbitrage opportunity detected! Sum of NO prices: {total_no_price:.4} (opportunity: ${arbitrage_amount:.4})"
                        ),
                        market_data: HashMap::new(),
                    };
                    alerts.push(alert);
                }
            }
        }

        self.last_arbitrage_check = Some(now);
        alerts
    }

    fn get_event_markets(&self, event_id: &str) -> Vec<String> {
        // For now, we'll use a simple approach - return all selected market IDs
        // In a more sophisticated implementation, we'd need to track which markets
        // belong to which events
        if self.selected_event_ids.contains(&event_id.to_string()) {
            self.selected_market_ids.clone()
        } else {
            Vec::new()
        }
    }
}
