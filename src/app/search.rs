//! Search functionality for markets and events

use super::core::App;

impl App {
    pub fn add_search_char(&mut self, ch: char) {
        self.search_query.push(ch);
        self.reset_search_state();
    }

    pub fn remove_search_char(&mut self) {
        self.search_query.pop();
        self.reset_search_state();
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.reset_search_state();
    }

    pub fn toggle_search_mode(&mut self) {
        self.search_mode = !self.search_mode;
        if !self.search_mode {
            self.clear_search();
        }
        self.needs_redraw = true;
    }

    /// Resets search-related state after any search query change
    fn reset_search_state(&mut self) {
        self.update_filtered_markets();
        self.update_filtered_events();
        self.selected_market = 0;
        self.selected_event = 0;
        self.market_scroll_offset = 0;
        self.event_scroll_offset = 0;
        self.needs_redraw = true;
    }
}
