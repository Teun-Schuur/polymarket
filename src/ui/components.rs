use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use crate::app::App;
use crate::data::{OrderBookData};

pub fn render_tab_bar(f: &mut Frame, app: &App, area: Rect) {
    let tab_titles = vec!["Orderbook", "Price History"];
    let selected_tab_index = match app.selected_tab {
        crate::app::SelectedTab::Orderbook => 0,
        crate::app::SelectedTab::PriceHistory => 1,
    };
    
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .select(selected_tab_index)
        .divider("|");
        
    f.render_widget(tabs, area);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn render_combined_market_header(f: &mut Frame, orderbook: &OrderBookData, ws_status: &str, area: Rect) {
    // Calculate decimal places based on tick size
    let decimal_places = if orderbook.tick_size >= 1.0 {
        0
    } else {
        (-orderbook.tick_size.log10().floor() as usize).min(6)
    };

    // Truncate market question if too long
    let market_question = if orderbook.market_question.len() > 60 {
        format!("{}...", &orderbook.market_question[..57])
    } else {
        orderbook.market_question.clone()
    };

    // Create a combined info line with market name and key stats
    let combined_info = format!(
        "{market_question} | Spread: {spread:.decimal_places$} | Tick: {tick_size:.decimal_places$} | Updated: {last_updated} | {ws_status}",
        market_question = market_question,
        spread = orderbook.get_spread(),
        tick_size = orderbook.tick_size,
        last_updated = orderbook.last_updated.format("%H:%M:%S UTC"),
        ws_status = ws_status,
        decimal_places = decimal_places
    );
    
    let header = Paragraph::new(combined_info)
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center)
        .block(Block::default()
            .borders(Borders::ALL)
            .title("Market Information"));
    
    f.render_widget(header, area);
}
