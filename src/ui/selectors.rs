use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};
use cli_log::warn;

use crate::app::{App, MarketSelectorTab};

pub fn render_market_selector(f: &mut Frame, app: &App, area: Rect) {
    // Split area for tabs and content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Content
        ])
        .split(area);

    // Render tabs
    let tab_titles = vec!["All Markets", "Events"];
    let selected_tab_index = match app.market_selector_tab {
        MarketSelectorTab::AllMarkets => 0,
        MarketSelectorTab::Events => 1,
    };
    
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title("Market Selector"))
        .select(selected_tab_index)
        .style(Style::default())
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    
    f.render_widget(tabs, chunks[0]);
    
    // Render content based on selected tab
    match app.market_selector_tab {
        MarketSelectorTab::AllMarkets => render_all_markets_list(f, app, chunks[1]),
        MarketSelectorTab::Events => render_events_list(f, app, chunks[1]),
    }
}

fn render_all_markets_list(f: &mut Frame, app: &App, area: Rect) {
    // Calculate visible area for scrolling
    let visible_height = area.height.saturating_sub(3) as usize; // Account for borders and title
    let total_items = app.filtered_markets.len();
    
    if total_items == 0 {
        let empty_list = List::new(vec![ListItem::new("No markets found")])
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Markets (0 total)"));
        f.render_widget(empty_list, area);
        return;
    }
    
    // Calculate scroll offset to keep selected item visible
    let scroll_offset = if app.selected_market >= visible_height {
        app.selected_market - visible_height + 1
    } else {
        0
    };
    
    let visible_start = scroll_offset;
    let visible_end = std::cmp::min(visible_start + visible_height, total_items);
    
    // Pre-allocate the items vector for better performance
    let mut items = Vec::with_capacity(visible_height);
    
    for (i, &market_idx) in app.filtered_markets
        .iter()
        .skip(visible_start)
        .take(visible_height)
        .enumerate() 
    {
        let market = &app.markets[market_idx];
        let global_idx = visible_start + i;
        let style = if global_idx == app.selected_market {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        
        // Cache the formatted string to avoid repeated allocations
        let text = market.question.clone();
        items.push(ListItem::new(Line::from(vec![Span::styled(text, style)])));
    }

    // Cache the title string
    let title = if app.search_mode {
        format!("Markets - Search: '{}' ({}/{})", app.search_query, app.filtered_markets.len(), app.markets.len())
    } else {
        format!("Markets ({} total)", app.markets.len())
    };

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(if app.search_mode { 
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) 
            } else { 
                Style::default() 
            }))
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(list, area);
    
    // Show scroll indicator if needed
    if total_items > visible_height {
        let scroll_indicator = format!(" {}-{}/{} ", 
            visible_start + 1, 
            visible_end, 
            total_items
        );
        let indicator_width = scroll_indicator.len() as u16;
        if indicator_width < area.width {
            let indicator_area = Rect {
                x: area.x + area.width - indicator_width - 1,
                y: area.y,
                width: indicator_width,
                height: 1,
            };
            let indicator = Paragraph::new(scroll_indicator)
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(indicator, indicator_area);
        }
    }
}

fn render_events_list(f: &mut Frame, app: &App, area: Rect) {
    // Calculate visible area for scrolling
    let visible_height = area.height.saturating_sub(3) as usize; // Account for borders and title
    let total_items = app.filtered_events.len();
    
    if total_items == 0 {
        let empty_list = List::new(vec![ListItem::new("No events found")])
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Events (0 total)"));
        f.render_widget(empty_list, area);
        return;
    }
    
    // Calculate scroll offset to keep selected item visible
    let scroll_offset = if app.selected_event >= visible_height {
        app.selected_event - visible_height + 1
    } else {
        0
    };
    
    let visible_start = scroll_offset;
    let visible_end = std::cmp::min(visible_start + visible_height, total_items);
    
    // Pre-allocate the items vector for better performance
    let mut items = Vec::with_capacity(visible_height);
    
    for (i, &event_idx) in app.filtered_events
        .iter()
        .skip(visible_start)
        .take(visible_height)
        .enumerate() 
    {
        let event = &app.events[event_idx];
        let global_idx = visible_start + i;
        let style = if global_idx == app.selected_event {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        
        // Show event title and number of markets
        let market_count = event.markets.as_ref().map(|m| m.len()).unwrap_or(0);
        let text = format!("{} ({} markets)", event.title, market_count);
        items.push(ListItem::new(Line::from(vec![Span::styled(text, style)])));
    }

    // Cache the title string
    let title = if app.search_mode {
        format!("Events - Search: '{}' ({}/{})", app.search_query, app.filtered_events.len(), app.events.len())
    } else {
        format!("Events ({} total)", app.events.len())
    };

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(if app.search_mode { 
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) 
            } else { 
                Style::default() 
            }))
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(list, area);
    
    // Show scroll indicator if needed
    if total_items > visible_height {
        let scroll_indicator = format!(" {}-{}/{} ", 
            visible_start + 1, 
            visible_end, 
            total_items
        );
        let indicator_width = scroll_indicator.len() as u16;
        if indicator_width < area.width {
            let indicator_area = Rect {
                x: area.x + area.width - indicator_width - 1,
                y: area.y,
                width: indicator_width,
                height: 1,
            };
            let indicator = Paragraph::new(scroll_indicator)
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(indicator, indicator_area);
        }
    }
}

pub fn render_token_selector(f: &mut Frame, app: &App, area: Rect) {
    if app.filtered_markets.is_empty() || app.selected_market >= app.filtered_markets.len() {
        warn!("No market selected or filtered markets are empty");
        return;
    }
    
    let market_idx = app.filtered_markets[app.selected_market];
    let market = &app.markets[market_idx];
    
    if market.tokens.is_empty() {
        let empty_list = List::new(vec![ListItem::new("No tokens found")])
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Tokens (0 total)"));
        f.render_widget(empty_list, area);
        return;
    }
    
    // Since there are typically always 2 tokens, let's optimize for that
    let total_items = market.tokens.len();
    
    // Pre-allocate the items vector
    let mut items = Vec::with_capacity(total_items);
    
    for (i, token) in market.tokens.iter().enumerate() {
        let style = if i == app.selected_token {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        
        // Show the token outcome with some additional formatting for prediction markets
        let token_text = if total_items == 2 {
            // For binary markets, show clear Yes/No or similar
            match i {
                0 => format!("► {}", token.outcome),
                1 => format!("► {}", token.outcome),
                _ => token.outcome.clone(),
            }
        } else {
            format!("{}. {}", i + 1, token.outcome)
        };
        
        items.push(ListItem::new(Line::from(vec![Span::styled(
            token_text,
            style,
        )])));
    }

    // Cache the title string with better formatting for binary markets
    let title = if market.question.len() > 40 {
        format!("Select Outcome - {}... ({} options)", &market.question[..37], total_items)
    } else {
        format!("Select Outcome - {} ({} options)", market.question, total_items)
    };

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(list, area);
    
    // Since there are typically only 2 tokens, no need for scroll indicators
    // But add them if there are more than can fit
    let visible_height = area.height.saturating_sub(3) as usize;
    if total_items > visible_height {
        let scroll_indicator = format!(" {} of {} ", 
            app.selected_token + 1, 
            total_items
        );
        let indicator_width = scroll_indicator.len() as u16;
        if indicator_width < area.width {
            let indicator_area = Rect {
                x: area.x + area.width - indicator_width - 1,
                y: area.y,
                width: indicator_width,
                height: 1,
            };
            let indicator = Paragraph::new(scroll_indicator)
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(indicator, indicator_area);
        }
    }
}

pub fn render_event_market_selector(f: &mut Frame, app: &App, area: Rect) {
    if app.filtered_events.is_empty() || app.selected_event >= app.filtered_events.len() {
        warn!("No event selected or filtered events are empty");
        return;
    }
    
    let event_idx = app.filtered_events[app.selected_event];
    let event = &app.events[event_idx];
    
    let markets = match &event.markets {
        Some(markets) => markets,
        None => {
            let empty_list = List::new(vec![ListItem::new("No markets found in this event")])
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Markets (0 total)"));
            f.render_widget(empty_list, area);
            return;
        }
    };
    
    if markets.is_empty() {
        let empty_list = List::new(vec![ListItem::new("No markets found in this event")])
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Markets (0 total)"));
        f.render_widget(empty_list, area);
        return;
    }
    
    // Calculate visible area for scrolling
    let visible_height = area.height.saturating_sub(3) as usize;
    let total_items = markets.len();
    
    // Calculate scroll offset to keep selected item visible
    let scroll_offset = if app.selected_market >= visible_height {
        app.selected_market - visible_height + 1
    } else {
        0
    };
    
    let visible_start = scroll_offset;
    let visible_end = std::cmp::min(visible_start + visible_height, total_items);
    
    // Pre-allocate the items vector
    let mut items = Vec::with_capacity(visible_height);
    
    for (i, market) in markets
        .iter()
        .skip(visible_start)
        .take(visible_height)
        .enumerate() 
    {
        let global_idx = visible_start + i;
        let style = if global_idx == app.selected_market {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        
        // Show market question
        let text = market.question.clone();
        items.push(ListItem::new(Line::from(vec![Span::styled(text, style)])));
    }

    // Cache the title string
    let title = if event.title.len() > 30 {
        format!("Markets in: {}... ({} total)", &event.title[..27], total_items)
    } else {
        format!("Markets in: {} ({} total)", event.title, total_items)
    };

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(list, area);
    
    // Show scroll indicator if needed
    if total_items > visible_height {
        let scroll_indicator = format!(" {}-{}/{} ", 
            visible_start + 1, 
            visible_end, 
            total_items
        );
        let indicator_width = scroll_indicator.len() as u16;
        if indicator_width < area.width {
            let indicator_area = Rect {
                x: area.x + area.width - indicator_width - 1,
                y: area.y,
                width: indicator_width,
                height: 1,
            };
            let indicator = Paragraph::new(scroll_indicator)
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(indicator, indicator_area);
        }
    }
}

pub fn render_event_token_selector(f: &mut Frame, app: &App, area: Rect) {
    if app.filtered_events.is_empty() || app.selected_event >= app.filtered_events.len() {
        warn!("No event selected or filtered events are empty");
        return;
    }
    
    let event_idx = app.filtered_events[app.selected_event];
    let event = &app.events[event_idx];
    
    let markets = match &event.markets {
        Some(markets) => markets,
        None => {
            let empty_list = List::new(vec![ListItem::new("No markets found")])
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Tokens (0 total)"));
            f.render_widget(empty_list, area);
            return;
        }
    };
    
    if app.selected_market >= markets.len() {
        warn!("Selected market index out of bounds");
        return;
    }
    
    let market = &markets[app.selected_market];
    let token_ids = match &market.clob_token_ids {
        Some(tokens) => tokens,
        None => {
            let empty_list = List::new(vec![ListItem::new("No tokens found")])
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Tokens (0 total)"));
            f.render_widget(empty_list, area);
            return;
        }
    };
    
    if token_ids.is_empty() {
        let empty_list = List::new(vec![ListItem::new("No tokens found")])
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Tokens (0 total)"));
        f.render_widget(empty_list, area);
        return;
    }
    
    let total_items = token_ids.len();
    
    // Pre-allocate the items vector
    let mut items = Vec::with_capacity(total_items);
    
    // For event markets, we use outcomes if available, otherwise show token IDs
    let outcomes = market.outcomes.as_ref();
    
    for (i, token_id) in token_ids.iter().enumerate() {
        let style = if i == app.selected_token {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        
        let token_text = if let Some(outcomes) = outcomes {
            if i < outcomes.len() {
                format!("► {}", outcomes[i])
            } else {
                format!("► Token {}: {}", i + 1, &token_id[..8])
            }
        } else {
            format!("► Token {}: {}", i + 1, &token_id[..8])
        };
        
        items.push(ListItem::new(Line::from(vec![Span::styled(
            token_text,
            style,
        )])));
    }

    // Cache the title string
    let title = if market.question.len() > 40 {
        format!("Select Outcome - {}... ({} options)", &market.question[..37], total_items)
    } else {
        format!("Select Outcome - {} ({} options)", market.question, total_items)
    };

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(list, area);
    
    // Show scroll indicator if needed
    let visible_height = area.height.saturating_sub(3) as usize;
    if total_items > visible_height {
        let scroll_indicator = format!(" {} of {} ", 
            app.selected_token + 1, 
            total_items
        );
        let indicator_width = scroll_indicator.len() as u16;
        if indicator_width < area.width {
            let indicator_area = Rect {
                x: area.x + area.width - indicator_width - 1,
                y: area.y,
                width: indicator_width,
                height: 1,
            };
            let indicator = Paragraph::new(scroll_indicator)
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(indicator, indicator_area);
        }
    }
}
