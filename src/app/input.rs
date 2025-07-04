use anyhow::Result;
use crossterm::event::KeyCode;
use crate::App;
use super::MarketSelectorTab;

impl App {
    pub async fn handle_key_input(&mut self, key_code: KeyCode) -> Result<bool> {
        match key_code {
            KeyCode::Char('q') => {
                if self.search_mode {
                    self.add_search_char('q');
                } else {
                    return Ok(false); // Exit
                }
            }
            KeyCode::Char('m') => {
                if self.search_mode {
                    self.add_search_char('m');
                } else {
                    self.show_market_selector = true;
                    self.show_token_selector = false;
                    self.search_mode = false;
                    self.needs_redraw = true;
                }
            }
            KeyCode::Char('r') => {
                if self.search_mode {
                    self.add_search_char('r');
                } else if let Some(ref orderbook) = self.orderbook {
                    let token_id = orderbook.token_id.clone();
                    self.load_orderbook(&token_id).await?;
                    self.needs_redraw = true;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.handle_left_navigation(key_code);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.handle_right_navigation(key_code);
            }
            KeyCode::Char('/') => {
                if self.search_mode {
                    self.add_search_char('/');
                } else if self.show_market_selector {
                    self.toggle_search_mode();
                }
            }
            KeyCode::Esc => {
                if self.search_mode {
                    self.toggle_search_mode();
                }
            }
            KeyCode::Up => self.handle_up_navigation(),
            KeyCode::Down => self.handle_down_navigation(),
            KeyCode::PageUp => self.handle_page_up(),
            KeyCode::PageDown => self.handle_page_down(),
            KeyCode::Enter => {
                self.handle_enter_selection().await?;
            }
            KeyCode::Backspace => {
                self.handle_backspace();
            }
            KeyCode::Char('s') => {
                if self.search_mode {
                    self.add_search_char('s');
                } else if self.show_strategy_runner {
                    // Start/stop strategy
                    if let Some(strategy_type) = self.get_current_strategy_type() {
                        if let Some(strategy) = self.bot_engine.get_strategy(&strategy_type) {
                            match strategy.status {
                                crate::bot::StrategyStatus::Running => self.stop_current_strategy(),
                                _ => { let _ = self.start_current_strategy(); }
                            }
                        }
                    }
                } else {
                    // Show strategy selector
                    self.show_market_selector = true;
                    self.show_strategy_selector = true;
                    self.market_selector_tab = MarketSelectorTab::Strategies;
                    self.needs_redraw = true;
                }
            }
            KeyCode::Char('a') => {
                if self.search_mode {
                    self.add_search_char('a');
                } else if self.show_strategy_runner {
                    // Add current market/event to strategy based on strategy scope
                    if let Some(strategy_type) = self.get_current_strategy_type() {
                        match strategy_type.scope() {
                            crate::bot::strategy::StrategyScope::Event => {
                                self.add_current_event_to_strategy();
                            }
                            crate::bot::strategy::StrategyScope::SingleMarket |
                            crate::bot::strategy::StrategyScope::MultiMarket => {
                                self.add_current_market_to_strategy();
                            }
                        }
                    }
                } else if self.show_market_selector && 
                          self.market_selector_tab == MarketSelectorTab::Events {
                    // Add event to strategy if we're in event selection mode
                    self.add_current_event_to_strategy();
                }
            }
            KeyCode::Char('p') => {
                if self.search_mode {
                    self.add_search_char('p');
                } else if self.show_strategy_runner {
                    // Pick markets/events for strategy
                    self.show_strategy_market_selector();
                }
            }
            KeyCode::Char(ch) => {
                if self.search_mode && !matches!(ch, 'q' | 'm' | 'r' | '/' | 's' | 'a') {
                    self.add_search_char(ch);
                }
            }
            _ => {}
        }
        Ok(true) // Continue running
    }

    fn handle_left_navigation(&mut self, key_code: KeyCode) {
        if self.search_mode {
            if matches!(key_code, KeyCode::Char('h')) {
                self.add_search_char('h');
            }
        } else if self.show_market_selector {
            self.previous_market_selector_tab();
        } else if !self.show_market_selector && !self.show_event_market_selector && !self.show_token_selector {
            self.previous_tab();
        }
    }

    fn handle_right_navigation(&mut self, key_code: KeyCode) {
        if self.search_mode {
            if matches!(key_code, KeyCode::Char('l')) {
                self.add_search_char('l');
            }
        } else if self.show_market_selector {
            self.next_market_selector_tab();
        } else if !self.show_market_selector && !self.show_event_market_selector && !self.show_token_selector {
            self.next_tab();
        }
    }

    fn handle_up_navigation(&mut self) {
        if self.show_market_selector {
            match self.market_selector_tab {
                MarketSelectorTab::AllMarkets => self.previous_market(),
                MarketSelectorTab::Events => self.previous_event(),
                MarketSelectorTab::Strategies => self.previous_strategy(),
            }
        } else if self.show_event_market_selector {
            self.previous_event_market();
        } else if self.show_token_selector {
            if self.market_selector_tab == MarketSelectorTab::Events {
                self.previous_event_token();
            } else {
                self.previous_token();
            }
        }
    }

    fn handle_down_navigation(&mut self) {
        if self.show_market_selector {
            match self.market_selector_tab {
                MarketSelectorTab::AllMarkets => self.next_market(),
                MarketSelectorTab::Events => self.next_event(),
                MarketSelectorTab::Strategies => self.next_strategy(),
            }
        } else if self.show_event_market_selector {
            self.next_event_market();
        } else if self.show_token_selector {
            if self.market_selector_tab == MarketSelectorTab::Events {
                self.next_event_token();
            } else {
                self.next_token();
            }
        }
    }

    fn handle_page_up(&mut self) {
        if self.show_market_selector {
            match self.market_selector_tab {
                MarketSelectorTab::AllMarkets => self.page_up_markets(),
                MarketSelectorTab::Events => self.page_up_events(),
                MarketSelectorTab::Strategies => {}, // No pagination for strategies
            }
        } else if self.show_token_selector {
            self.page_up_tokens();
        }
    }

    fn handle_page_down(&mut self) {
        if self.show_market_selector {
            match self.market_selector_tab {
                MarketSelectorTab::AllMarkets => self.page_down_markets(),
                MarketSelectorTab::Events => self.page_down_events(),
                MarketSelectorTab::Strategies => {}, // No pagination for strategies
            }
        } else if self.show_token_selector {
            self.page_down_tokens();
        }
    }

    async fn handle_enter_selection(&mut self) -> Result<()> {
        if self.show_market_selector {
            match self.market_selector_tab {
                MarketSelectorTab::AllMarkets => self.select_market(),
                MarketSelectorTab::Events => self.select_event(),
                MarketSelectorTab::Strategies => self.select_strategy(),
            }
        } else if self.show_event_market_selector {
            self.select_event_market();
        } else if self.show_token_selector {
            self.select_token();
            if let Some(token_id) = self.get_current_token_id() {
                cli_log::info!("Loading orderbook for token ID: {token_id}");
                self.load_orderbook(&token_id).await?;
                self.start_websocket_for_token(&token_id);
                self.needs_redraw = true;
            }
        }
        Ok(())
    }

    fn handle_backspace(&mut self) {
        if self.show_strategy_runner {
            // Go back to strategy selector
            self.show_strategy_runner = false;
            self.show_strategy_selector = true;
            self.show_market_selector = true;
            self.market_selector_tab = MarketSelectorTab::Strategies;
            self.bot_engine.active_strategy = None; // Clear active strategy
            self.needs_redraw = true;
        } else if self.strategy_selection_mode {
            // Go back to strategy runner from market/event selection
            self.show_market_selector = false;
            self.show_event_market_selector = false;
            self.show_token_selector = false;
            self.show_strategy_runner = true;
            self.strategy_selection_mode = false;
            self.needs_redraw = true;
        } else if self.show_market_selector && self.market_selector_tab == MarketSelectorTab::Strategies {
            // Go back to normal market selector from strategy selector
            self.show_strategy_selector = false;
            self.market_selector_tab = MarketSelectorTab::AllMarkets;
            self.needs_redraw = true;
        } else if self.show_token_selector {
            if self.market_selector_tab == MarketSelectorTab::Events {
                self.show_event_market_selector = true;
                self.show_token_selector = false;
            } else {
                self.show_market_selector = true;
                self.show_token_selector = false;
            }
            self.needs_redraw = true;
        } else if self.show_event_market_selector {
            self.show_market_selector = true;
            self.show_event_market_selector = false;
            self.needs_redraw = true;
        } else if self.search_mode {
            self.remove_search_char();
        }
    }
}
