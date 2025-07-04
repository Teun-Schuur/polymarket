//! Strategy selection and navigation functionality

use super::core::App;
use crate::bot::StrategyType;
use cli_log::*;

impl App {
    pub fn get_available_strategies(&self) -> Vec<StrategyType> {
        StrategyType::all_strategies()
    }

    pub fn get_current_strategy_type(&self) -> Option<StrategyType> {
        let strategies = self.get_available_strategies();
        strategies.get(self.selected_strategy).cloned()
    }

    pub fn next_strategy(&mut self) {
        let strategies = self.get_available_strategies();
        if !strategies.is_empty() {
            self.selected_strategy = (self.selected_strategy + 1) % strategies.len();
            self.needs_redraw = true;
        }
    }

    pub fn previous_strategy(&mut self) {
        let strategies = self.get_available_strategies();
        if !strategies.is_empty() {
            self.selected_strategy = if self.selected_strategy == 0 {
                strategies.len() - 1
            } else {
                self.selected_strategy - 1
            };
            self.needs_redraw = true;
        }
    }

    pub fn select_strategy(&mut self) {
        if let Some(strategy_type) = self.get_current_strategy_type() {
            self.bot_engine.active_strategy = Some(strategy_type);
            self.show_strategy_selector = false;
            self.show_strategy_runner = true;
            self.needs_redraw = true;
        }
    }

    pub fn start_current_strategy(&mut self) -> anyhow::Result<()> {
        if let Some(strategy_type) = self.get_current_strategy_type() {
            self.bot_engine.start_strategy(strategy_type.clone())?;
            info!("Started strategy: {}", strategy_type.name());
            self.set_status_message(format!("Started strategy: {}", strategy_type.name()));
            self.error_message = None; // Clear any previous error
        }
        Ok(())
    }

    pub fn stop_current_strategy(&mut self) {
        if let Some(strategy_type) = self.get_current_strategy_type() {
            self.bot_engine.stop_strategy(&strategy_type);
            info!("Stopped strategy: {}", strategy_type.name());
            self.set_status_message(format!("Stopped strategy: {}", strategy_type.name()));
            self.error_message = None; // Clear any previous error
        }
    }

    pub fn add_current_market_to_strategy(&mut self) {
        if let Some(strategy_type) = self.get_current_strategy_type() {
            match strategy_type.scope() {
                crate::bot::strategy::StrategyScope::SingleMarket | 
                crate::bot::strategy::StrategyScope::MultiMarket => {
                    // Try to get token ID from current orderbook first
                    if let Some(ref orderbook) = self.orderbook {
                        // Find the market that contains this token ID to get its name
                        let market_name = self.find_market_name_by_token_id(&orderbook.token_id)
                            .unwrap_or_else(|| format!("Market {}", &orderbook.token_id[..orderbook.token_id.len().min(8)]));
                        self.bot_engine.add_market_to_strategy(&strategy_type, orderbook.token_id.clone(), market_name);
                    }
                    // If no orderbook, try to get from selected market
                    else if !self.filtered_markets.is_empty() && self.selected_market < self.filtered_markets.len() {
                        let market_idx = self.filtered_markets[self.selected_market];
                        if let Some(market) = self.markets.get(market_idx) {
                            if let Some(token_id) = market.token_ids.first() {
                                self.bot_engine.add_market_to_strategy(&strategy_type, token_id.clone(), market.question.clone());
                            }
                        }
                    }
                }
                crate::bot::strategy::StrategyScope::Event => {
                    // For event-based strategies, we need the event ID
                    // This would need to be tracked in the orderbook or app state
                }
            }
            self.needs_redraw = true;
        }
    }

    pub fn add_current_event_to_strategy(&mut self) {
        if let Some(strategy_type) = self.get_current_strategy_type() {
            if matches!(strategy_type.scope(), crate::bot::strategy::StrategyScope::Event)
                && self.selected_event < self.filtered_events.len() {
                let event_index = self.filtered_events[self.selected_event];
                if let Some(event) = self.events.get(event_index) {
                    let event_id = event.id.clone();
                    let event_title = event.title.clone();
                    
                    self.bot_engine.add_event_to_strategy(&strategy_type, event_id.clone(), event_title.clone());
                    
                    // For arbitrage detector, also add all markets within the event
                    if strategy_type == crate::bot::StrategyType::ArbitrageDetector {
                        let markets_added = self.add_event_markets_to_strategy(&strategy_type, &event_id);
                        info!("Added {} markets from event '{}' to strategy '{}'", markets_added, event_title, strategy_type.name());
                        self.set_status_message(format!("Added event '{}' with {} markets to {}", event_title, markets_added, strategy_type.name()));
                    } else {
                        info!("Added event '{}' to strategy '{}'", event_title, strategy_type.name());
                        self.set_status_message(format!("Added event '{}' to {}", event_title, strategy_type.name()));
                    }
                }
            }
        }
    }

    pub fn show_strategy_market_selector(&mut self) {
        if let Some(strategy_type) = self.get_current_strategy_type() {
            match strategy_type.scope() {
                crate::bot::strategy::StrategyScope::Event => {
                    // Show event selector
                    self.show_market_selector = true;
                    self.show_strategy_runner = false;
                    self.strategy_selection_mode = true;
                    self.market_selector_tab = crate::app::MarketSelectorTab::Events;
                }
                crate::bot::strategy::StrategyScope::SingleMarket |
                crate::bot::strategy::StrategyScope::MultiMarket => {
                    // Show market selector
                    self.show_market_selector = true;
                    self.show_strategy_runner = false;
                    self.strategy_selection_mode = true;
                    self.market_selector_tab = crate::app::MarketSelectorTab::AllMarkets;
                }
            }
            self.needs_redraw = true;
        }
    }

    fn find_market_name_by_token_id(&self, token_id: &str) -> Option<String> {
        for market in &self.markets {
            if market.token_ids.contains(&token_id.to_string()) {
                return Some(market.question.clone());
            }
        }
        None
    }

    fn add_event_markets_to_strategy(&mut self, strategy_type: &crate::bot::StrategyType, event_id: &str) -> usize {
        let mut markets_added = 0;
        // Find the event and add all its markets to the strategy
        if let Some(event) = self.events.iter().find(|e| e.id == event_id) {
            if let Some(ref markets) = event.markets {
                for market in markets {
                    for token_id in &market.token_ids {
                        self.bot_engine.add_market_to_strategy(strategy_type, token_id.clone(), market.question.clone());
                        markets_added += 1;
                    }
                }
            }
        }
        markets_added
    }
}
