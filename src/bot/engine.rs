use anyhow::Result;
use chrono::Utc;
use cli_log::*;
use std::collections::HashMap;

use crate::data::OrderBookData;
use super::strategy::{Strategy, StrategyAlert, StrategyStatus, StrategyType, AlertSeverity};

pub struct BotEngine {
    pub strategies: HashMap<StrategyType, Strategy>,
    pub active_strategy: Option<StrategyType>,
}

impl Default for BotEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl BotEngine {
    pub fn new() -> Self {
        let mut strategies = HashMap::new();
        
        for strategy_type in StrategyType::all_strategies() {
            strategies.insert(strategy_type.clone(), Strategy::new(strategy_type));
        }

        Self {
            strategies,
            active_strategy: None,
        }
    }

    pub fn get_strategy(&self, strategy_type: &StrategyType) -> Option<&Strategy> {
        self.strategies.get(strategy_type)
    }

    pub fn get_strategy_mut(&mut self, strategy_type: &StrategyType) -> Option<&mut Strategy> {
        self.strategies.get_mut(strategy_type)
    }

    pub fn start_strategy(&mut self, strategy_type: StrategyType) -> Result<()> {
        info!("Starting strategy: {}", strategy_type.name());
        if let Some(strategy) = self.strategies.get_mut(&strategy_type) {
            strategy.status = StrategyStatus::Running;
            strategy.last_run = Some(Utc::now());
            self.active_strategy = Some(strategy_type.clone());
            info!("Strategy '{}' started successfully", strategy_type.name());
        }
        Ok(())
    }

    pub fn stop_strategy(&mut self, strategy_type: &StrategyType) {
        info!("Stopping strategy: {}", strategy_type.name());
        if let Some(strategy) = self.strategies.get_mut(strategy_type) {
            strategy.status = StrategyStatus::Stopped;
            info!("Strategy '{}' stopped", strategy_type.name());
        }
        if self.active_strategy.as_ref() == Some(strategy_type) {
            self.active_strategy = None;
        }
    }

    pub fn add_market_to_strategy(&mut self, strategy_type: &StrategyType, token_id: String, market_name: String) {
        if let Some(strategy) = self.strategies.get_mut(strategy_type) {
            if matches!(strategy.strategy_type.scope(), super::strategy::StrategyScope::SingleMarket | super::strategy::StrategyScope::MultiMarket)
                && !strategy.selected_market_ids.contains(&token_id) {
                strategy.selected_market_ids.push(token_id.clone());
                strategy.selected_market_names.push(market_name.clone());
                info!("Added market '{}' (ID: {}) to strategy '{}'", market_name, token_id, strategy_type.name());
            }
        }
    }

    pub fn add_event_to_strategy(&mut self, strategy_type: &StrategyType, event_id: String, event_name: String) {
        if let Some(strategy) = self.strategies.get_mut(strategy_type) {
            if matches!(strategy.strategy_type.scope(), super::strategy::StrategyScope::Event)
                && !strategy.selected_event_ids.contains(&event_id) {
                strategy.selected_event_ids.push(event_id.clone());
                strategy.selected_event_names.push(event_name.clone());
                info!("Added event '{}' (ID: {}) to strategy '{}'", event_name, event_id, strategy_type.name());
            }
        }
    }

    pub fn remove_market_from_strategy(&mut self, strategy_type: &StrategyType, token_id: &str) {
        if let Some(strategy) = self.strategies.get_mut(strategy_type) {
            if let Some(index) = strategy.selected_market_ids.iter().position(|id| id == token_id) {
                strategy.selected_market_ids.remove(index);
                if index < strategy.selected_market_names.len() {
                    strategy.selected_market_names.remove(index);
                }
            }
        }
    }

    pub fn remove_event_from_strategy(&mut self, strategy_type: &StrategyType, event_id: &str) {
        if let Some(strategy) = self.strategies.get_mut(strategy_type) {
            if let Some(index) = strategy.selected_event_ids.iter().position(|id| id == event_id) {
                strategy.selected_event_ids.remove(index);
                if index < strategy.selected_event_names.len() {
                    strategy.selected_event_names.remove(index);
                }
            }
        }
    }

    pub fn process_orderbook(&mut self, orderbook: &OrderBookData) -> Result<()> {
        let mut updates = Vec::new();
        
        // Collect strategies that need processing
        for (strategy_type, strategy) in &self.strategies {
            if matches!(strategy.status, StrategyStatus::Running)
                && strategy.selected_market_ids.contains(&orderbook.token_id) {
                updates.push(strategy_type.clone());
            }
        }
        
        if !updates.is_empty() {
            debug!("Processing orderbook for token {} - {} strategies interested", orderbook.token_id, updates.len());
        }
        
        // Process each strategy
        for strategy_type in updates {
            if let Some(strategy) = self.strategies.get_mut(&strategy_type) {
                // For arbitrage detector, update the orderbook cache
                if strategy_type == StrategyType::ArbitrageDetector {
                    strategy.update_orderbook(orderbook.clone());
                    let alerts = strategy.check_arbitrage_opportunities();
                    if !alerts.is_empty() {
                        info!("Arbitrage detector found {} opportunity(ies)!", alerts.len());
                        for alert in &alerts {
                            info!("Alert: {}", alert.message);
                        }
                    }
                    strategy.alerts.extend(alerts);
                } else {
                    Self::run_strategy_analysis_static(&strategy_type, strategy, orderbook)?;
                }
            }
        }
        
        Ok(())
    }

    fn run_strategy_analysis_static(
        strategy_type: &StrategyType,
        strategy: &mut Strategy,
        orderbook: &OrderBookData,
    ) -> Result<()> {
        strategy.run_count += 1;
        strategy.last_run = Some(Utc::now());

        // Basic strategy implementations - these would be expanded
        match strategy_type {
            StrategyType::PriceAnomaly => {
                Self::analyze_price_anomaly_static(strategy, orderbook)?;
            }
            StrategyType::VolumeSpike => {
                Self::analyze_volume_spike_static(strategy, orderbook)?;
            }
            StrategyType::ArbitrageDetector => {
                // Requires multiple markets - implemented when we have market data
            }
            StrategyType::CrossMarketCorrelation => {
                // Requires multiple markets - implemented when we have market data
            }
        }

        Ok(())
    }

    fn analyze_price_anomaly_static(strategy: &mut Strategy, orderbook: &OrderBookData) -> Result<()> {
        let spread = orderbook.get_spread();
        let midpoint = orderbook.get_midpoint();

        // Simple anomaly detection: unusually wide spread
        if spread > 0.1 { // 10% spread threshold
            let alert = StrategyAlert {
                timestamp: Utc::now(),
                strategy: "Price Anomaly".to_string(),
                severity: if spread > 0.2 { AlertSeverity::High } else { AlertSeverity::Medium },
                message: format!(
                    "Wide spread detected: {:.2}% at midpoint {:.4}",
                    spread * 100.0,
                    midpoint
                ),
                market_data: std::collections::HashMap::new(),
            };
            strategy.alerts.push(alert);
            
            // Keep only last 100 alerts
            if strategy.alerts.len() > 100 {
                strategy.alerts.remove(0);
            }
        }

        Ok(())
    }

    fn analyze_volume_spike_static(strategy: &mut Strategy, orderbook: &OrderBookData) -> Result<()> {
        let total_bid_volume: f64 = orderbook.bids.iter().map(|b| b.size).sum();
        let total_ask_volume: f64 = orderbook.asks.iter().map(|a| a.size).sum();
        let total_volume = total_bid_volume + total_ask_volume;

        // Simple volume spike detection: total volume > threshold
        if total_volume > 10000.0 { // Arbitrary threshold
            let alert = StrategyAlert {
                timestamp: Utc::now(),
                strategy: "Volume Spike".to_string(),
                severity: if total_volume > 50000.0 { AlertSeverity::High } else { AlertSeverity::Medium },                    message: format!(
                        "Volume spike detected: {total_volume:.0} total volume (bids: {total_bid_volume:.0}, asks: {total_ask_volume:.0})"
                    ),
                market_data: std::collections::HashMap::new(),
            };
            strategy.alerts.push(alert);
            
            // Keep only last 100 alerts
            if strategy.alerts.len() > 100 {
                strategy.alerts.remove(0);
            }
        }

        Ok(())
    }

    pub fn get_strategy_status(&self, strategy_type: &StrategyType) -> Option<String> {
        self.strategies.get(strategy_type).map(|strategy| {
            match &strategy.status {
                StrategyStatus::Stopped => "Stopped".to_string(),
                StrategyStatus::Running => {
                    let markets_count = strategy.selected_market_ids.len();
                    let events_count = strategy.selected_event_ids.len();
                    let alerts_count = strategy.alerts.len();
                    
                    match strategy_type {
                        StrategyType::ArbitrageDetector => {
                            format!("Running - {events_count} events, {markets_count} markets, {alerts_count} alerts")
                        }
                        _ => {
                            format!("Running - {markets_count} markets, {alerts_count} alerts")
                        }
                    }
                }
                StrategyStatus::Error(err) => format!("Error: {err}"),
            }
        })
    }
}
