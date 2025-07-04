use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use super::{selectors::{render_market_selector, render_token_selector, render_event_market_selector, render_event_token_selector}, orderbook::render_orderbook, charts::render_market_price_history, components::{render_tab_bar, centered_rect}};

pub fn render_ui(f: &mut Frame, app: &mut App) {
    if app.show_strategy_runner {
        // Strategy runner view - full screen
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Header
        let header_text = if let Some(strategy_type) = app.bot_engine.active_strategy.as_ref() {
            format!("Strategy Runner - {}", strategy_type.name())
        } else {
            "Strategy Runner".to_string()
        };
        let header = Paragraph::new(header_text)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Strategy runner content
        crate::ui::strategies::render_strategy_runner(f, app, chunks[1]);

        // Footer with controls
        let footer_text = if let Some(strategy_type) = app.bot_engine.active_strategy.as_ref() {
            let strategy_status = app.bot_engine.strategies.get(strategy_type)
                .map(|s| match s.status {
                    crate::bot::StrategyStatus::Running => "RUNNING",
                    crate::bot::StrategyStatus::Stopped => "STOPPED",
                    crate::bot::StrategyStatus::Error(_) => "ERROR",
                })
                .unwrap_or("UNKNOWN");
            format!("Status: {} | [S] Start/Stop | [P] Pick Markets/Events | [Backspace] Back | [Q] Quit", strategy_status)
        } else {
            "No strategy selected | [Backspace] Back to Strategy Selector | [Q] Quit".to_string()
        };
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    } else if app.show_strategy_selector {
        // Strategy selector view
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Header
        let header = Paragraph::new("Strategy Selector")
            .style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Strategy selector content
        crate::ui::strategies::render_strategy_selector(f, app, chunks[1]);

        // Footer
        let footer = Paragraph::new("↑↓: Navigate | Enter: Select | Backspace: Back | q: Quit")
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    } else if app.show_market_selector || app.show_event_market_selector || app.show_token_selector {
        // Show header when in selectors
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Header
        let header = Paragraph::new("Polymarket Real-time Orderbook Viewer")
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Main content
        if app.show_market_selector {
            render_market_selector(f, app, chunks[1]);
        } else if app.show_event_market_selector {
            render_event_market_selector(f, app, chunks[1]);
        } else if app.show_token_selector {
            // Check if we're in event mode or regular market mode
            if app.market_selector_tab == crate::app::MarketSelectorTab::Events {
                render_event_token_selector(f, app, chunks[1]);
            } else {
                render_token_selector(f, app, chunks[1]);
            }
        }

        // Footer
        let footer_text = if app.show_market_selector {
            if app.search_mode {
                format!("Search: {} | ↑↓: Navigate | Tab: Switch tabs | PgUp/PgDn: Fast scroll | Enter: Select | Esc: Exit search | q: Quit", app.search_query)
            } else {
                "↑↓: Navigate | Tab: Switch tabs | PgUp/PgDn: Fast scroll | Enter: Select | /: Search | q: Quit".to_string()
            }
        } else if app.show_event_market_selector {
            "↑↓: Navigate | Enter: Select | Backspace: Back to Events | q: Quit".to_string()
        } else {
            "↑↓: Navigate | PgUp/PgDn: Fast scroll | Enter: Select | Backspace: Back | q: Quit".to_string()
        };
        
        let footer = Paragraph::new(footer_text.as_str())
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    } else {
        // Orderbook view with tabs - use full area with tabs, content, and footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(10),   // Main content
                Constraint::Length(3), // Footer
            ])
            .split(f.area());

        // Render tab bar
        render_tab_bar(f, app, chunks[0]);

        // Render content based on selected tab
        match app.selected_tab {
            crate::app::SelectedTab::Orderbook => {
                render_orderbook(f, app, chunks[1]);
            }
            crate::app::SelectedTab::PriceHistory => {
                render_market_price_history(f, app, chunks[1]);
            }
        }

        let footer = Paragraph::new("◄►/hl: Switch tabs | m: Market Selector | r: Refresh | q: Quit")
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    }

    // Error overlay
    if let Some(ref error) = app.error_message {
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area);
        let error_block = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Error")
                .style(Style::default().fg(Color::Red)));
        f.render_widget(error_block, area);
    }

    // Status message overlay
    if let Some(ref status) = app.status_message {
        let area = centered_rect(60, 15, f.area());
        f.render_widget(Clear, area);
        let status_block = Paragraph::new(status.as_str())
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Status")
                .style(Style::default().fg(Color::Green)));
        f.render_widget(status_block, area);
    }
}
