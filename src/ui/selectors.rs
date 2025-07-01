use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use cli_log::warn;

use crate::app::App;

pub fn render_market_selector(f: &mut Frame, app: &App, area: Rect) {
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
        let text = format!("{} ({} outcomes)", market.question, market.tokens.len());
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
