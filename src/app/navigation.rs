//! Navigation logic for markets, events, and tokens

use super::core::App;
use super::types::MarketSelectorTab;

impl App {
    // Basic navigation methods
    pub fn select_market(&mut self) {
        if !self.filtered_markets.is_empty() && self.selected_market < self.filtered_markets.len() {
            // Check if we're in strategy selection mode
            if self.strategy_selection_mode {
                // Add the current market to the strategy and return to runner
                self.add_current_market_to_strategy();
                self.show_market_selector = false;
                self.show_strategy_runner = true;
                self.strategy_selection_mode = false;
                self.search_mode = false;
                self.needs_redraw = true;
            } else {
                // Normal market selection - go to token selector
                self.selected_token = 0;
                self.token_scroll_offset = 0;
                self.show_market_selector = false;
                self.show_token_selector = true;
                self.search_mode = false;
                self.needs_redraw = true;
            }
        }
    }

    pub fn select_token(&mut self) {
        if !self.filtered_markets.is_empty() && self.selected_market < self.filtered_markets.len() {
            let market_idx = self.filtered_markets[self.selected_market];
            let market = &self.markets[market_idx];
            if self.selected_token < market.token_ids.len() {
                self.show_token_selector = false;
                self.needs_redraw = true;
                // We'll load the orderbook in the main loop
            }
        }
    }

    // Market navigation
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

    // Token navigation
    pub fn next_token(&mut self) {
        if !self.filtered_markets.is_empty() && self.selected_market < self.filtered_markets.len() {
            let market_idx = self.filtered_markets[self.selected_market];
            let market = &self.markets[market_idx];
            if !market.token_ids.is_empty() {
                self.selected_token = (self.selected_token + 1) % market.token_ids.len();
                self.needs_redraw = true;
            }
        }
    }

    pub fn previous_token(&mut self) {
        if !self.filtered_markets.is_empty() && self.selected_market < self.filtered_markets.len() {
            let market_idx = self.filtered_markets[self.selected_market];
            let market = &self.markets[market_idx];
            if !market.token_ids.is_empty() {
                self.selected_token = if self.selected_token == 0 {
                    market.token_ids.len() - 1
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
            if !market.token_ids.is_empty() {
                let page_size = 10; // Adjust based on terminal height
                self.selected_token = std::cmp::min(
                    self.selected_token + page_size,
                    market.token_ids.len() - 1
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
        // If we're in event market selector mode, or we came from the event selector path
        // (check if we're on the Events tab), get token from event market
        if self.show_event_market_selector || 
           (self.market_selector_tab == MarketSelectorTab::Events && !self.show_market_selector) {
            cli_log::debug!("Using event market logic for token ID retrieval");
            return self.get_current_event_market_token_id();
        }
        
        cli_log::debug!("Using regular market logic for token ID retrieval");
        
        // Otherwise, get token from regular markets
        if !self.filtered_markets.is_empty() && self.selected_market < self.filtered_markets.len() {
            let market_idx = self.filtered_markets[self.selected_market];
            let market = &self.markets[market_idx];
            if self.selected_token < market.token_ids.len() {
                return Some(market.token_ids[self.selected_token].clone());
            }
        }
        None
    }

    // Tab navigation
    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
        self.needs_redraw = true;
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
        self.needs_redraw = true;
    }

    // Market selector tab navigation
    pub fn next_market_selector_tab(&mut self) {
        self.market_selector_tab = self.market_selector_tab.next();
        self.needs_redraw = true;
    }

    pub fn previous_market_selector_tab(&mut self) {
        self.market_selector_tab = self.market_selector_tab.previous();
        self.needs_redraw = true;
    }

    // Event navigation
    pub fn select_event(&mut self) {
        if !self.filtered_events.is_empty() && self.selected_event < self.filtered_events.len() {
            // Check if we're in strategy selection mode
            if self.strategy_selection_mode {
                // Add the current event to the strategy and return to runner
                self.add_current_event_to_strategy();
                self.show_market_selector = false;
                self.show_strategy_runner = true;
                self.strategy_selection_mode = false;
                self.search_mode = false;
                self.needs_redraw = true;
            } else {
                // Normal event selection - go to event markets
                self.show_market_selector = false;
                self.show_event_market_selector = true;
                self.selected_token = 0;
                self.token_scroll_offset = 0;
                self.search_mode = false;
                self.needs_redraw = true;
            }
        }
    }

    pub fn select_event_market(&mut self) {
        if self.get_current_event_market_token_id().is_some() {
            self.show_event_market_selector = false;
            self.show_token_selector = true;  // Show token selector so user can choose Yes/No
            self.selected_token = 0;  // Reset to first token (usually "Yes")
            self.needs_redraw = true;
            // Note: token_id will be used to load orderbook in main loop after token selection
        }
    }

    pub fn get_current_event_market_token_id(&self) -> Option<String> {
        if !self.filtered_events.is_empty() && self.selected_event < self.filtered_events.len() {
            let event_idx = self.filtered_events[self.selected_event];
            let event = &self.events[event_idx];
            
            if let Some(ref markets) = event.markets {
                if self.selected_market < markets.len() {
                    let market = &markets[self.selected_market];
                    if self.selected_token < market.token_ids.len() {
                        cli_log::debug!("Event market token retrieval: event={}, market={}, token={}, token_id={}", 
                                        self.selected_event, self.selected_market, self.selected_token, market.token_ids[self.selected_token]);
                        return Some(market.token_ids[self.selected_token].clone());
                    }
                }
            }
        }
        cli_log::debug!("Event market token retrieval failed: no valid token found");
        None
    }

    pub fn next_event(&mut self) {
        if !self.filtered_events.is_empty() {
            self.selected_event = (self.selected_event + 1) % self.filtered_events.len();
            self.needs_redraw = true;
        }
    }

    pub fn previous_event(&mut self) {
        if !self.filtered_events.is_empty() {
            self.selected_event = if self.selected_event == 0 {
                self.filtered_events.len() - 1
            } else {
                self.selected_event - 1
            };
            self.needs_redraw = true;
        }
    }

    pub fn page_down_events(&mut self) {
        if !self.filtered_events.is_empty() {
            let page_size = 10; // Adjust based on terminal height
            self.selected_event = std::cmp::min(
                self.selected_event + page_size,
                self.filtered_events.len() - 1
            );
            self.needs_redraw = true;
        }
    }

    pub fn page_up_events(&mut self) {
        let page_size = 10; // Adjust based on terminal height
        self.selected_event = self.selected_event.saturating_sub(page_size);
        self.needs_redraw = true;
    }

    pub fn next_event_market(&mut self) {
        if !self.filtered_events.is_empty() && self.selected_event < self.filtered_events.len() {
            let event_idx = self.filtered_events[self.selected_event];
            let event = &self.events[event_idx];
            
            if let Some(ref markets) = event.markets {
                if !markets.is_empty() {
                    self.selected_market = (self.selected_market + 1) % markets.len();
                    self.selected_token = 0; // Reset token selection when switching markets
                    self.needs_redraw = true;
                }
            }
        }
    }

    pub fn previous_event_market(&mut self) {
        if !self.filtered_events.is_empty() && self.selected_event < self.filtered_events.len() {
            let event_idx = self.filtered_events[self.selected_event];
            let event = &self.events[event_idx];
            
            if let Some(ref markets) = event.markets {
                if !markets.is_empty() {
                    self.selected_market = if self.selected_market == 0 {
                        markets.len() - 1
                    } else {
                        self.selected_market - 1
                    };
                    self.selected_token = 0; // Reset token selection when switching markets
                    self.needs_redraw = true;
                }
            }
        }
    }

    pub fn next_event_token(&mut self) {
        if !self.filtered_events.is_empty() && self.selected_event < self.filtered_events.len() {
            let event_idx = self.filtered_events[self.selected_event];
            let event = &self.events[event_idx];
            
            if let Some(ref markets) = event.markets {
                if self.selected_market < markets.len() {
                    let market = &markets[self.selected_market];
                    if !market.token_ids.is_empty() {
                        self.selected_token = (self.selected_token + 1) % market.token_ids.len();
                        self.needs_redraw = true;
                    }
                }
            }
        }
    }

    pub fn previous_event_token(&mut self) {
        if !self.filtered_events.is_empty() && self.selected_event < self.filtered_events.len() {
            let event_idx = self.filtered_events[self.selected_event];
            let event = &self.events[event_idx];
            
            if let Some(ref markets) = event.markets {
                if self.selected_market < markets.len() {
                    let market = &markets[self.selected_market];
                    if !market.token_ids.is_empty() {
                        self.selected_token = if self.selected_token == 0 {
                            market.token_ids.len() - 1
                        } else {
                            self.selected_token - 1
                        };
                        self.needs_redraw = true;
                    }
                    
                }
            }
        }
    }
}
