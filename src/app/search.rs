//! Search functionality for markets and events

use super::core::App;

impl App {
    pub fn add_search_char(&mut self, ch: char) {
        self.search_query.push(ch);
        self.update_filtered_markets();
        self.update_filtered_events();
        self.selected_market = 0;
        self.selected_event = 0;
        self.market_scroll_offset = 0;
        self.event_scroll_offset = 0;
        self.needs_redraw = true;
    }

    pub fn remove_search_char(&mut self) {
        self.search_query.pop();
        self.update_filtered_markets();
        self.update_filtered_events();
        self.selected_market = 0;
        self.selected_event = 0;
        self.market_scroll_offset = 0;
        self.event_scroll_offset = 0;
        self.needs_redraw = true;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.update_filtered_markets();
        self.update_filtered_events();
        self.selected_market = 0;
        self.selected_event = 0;
        self.market_scroll_offset = 0;
        self.event_scroll_offset = 0;
        self.needs_redraw = true;
    }

    pub fn toggle_search_mode(&mut self) {
        self.search_mode = !self.search_mode;
        if !self.search_mode {
            self.clear_search();
        }
        self.needs_redraw = true;
    }
}
