use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::{
    app::App,
};

pub fn render_strategy_selector(f: &mut Frame, app: &App, area: Rect) {
    let strategies = app.get_available_strategies();
    
    let items: Vec<ListItem> = strategies
        .iter()
        .enumerate()
        .map(|(i, strategy_type)| {
            let style = if i == app.selected_strategy {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(strategy_type.name(), style),
                ]),
                Line::from(vec![
                    Span::styled(
                        format!("  {}", strategy_type.description()),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("Trading Strategies")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(list, area);
}

pub fn render_strategy_runner(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Selected markets/events
            Constraint::Min(0),    // Alerts/logs
        ])
        .split(area);

    if let Some(strategy_type) = app.get_current_strategy_type() {
        let strategy = app.bot_engine.get_strategy(&strategy_type).unwrap();

        // Selected markets/events
        let selection_title = match strategy_type.scope() {
            crate::bot::strategy::StrategyScope::Event => "Selected Events",
            _ => "Selected Markets",
        };

        let selection_items: Vec<ListItem> = match strategy_type.scope() {
            crate::bot::strategy::StrategyScope::Event => {
                if strategy.selected_event_ids.is_empty() {
                    vec![ListItem::new("No events selected - Press [P] to pick events")]
                } else {
                    strategy
                        .selected_event_ids
                        .iter()
                        .zip(&strategy.selected_event_names)
                        .map(|(event_id, event_name)| {
                            let truncated_id = if event_id.len() > 8 {
                                format!("{}...", &event_id[..8])
                            } else {
                                event_id.clone()
                            };
                            let truncated_name = if event_name.len() > 60 {
                                format!("{}...", &event_name[..57])
                            } else {
                                event_name.clone()
                            };
                            ListItem::new(vec![
                                Line::from(vec![
                                    Span::styled(truncated_id, Style::default().fg(Color::Cyan)),
                                    Span::raw(" - "),
                                    Span::styled(truncated_name, Style::default().fg(Color::White)),
                                ])
                            ])
                        })
                        .collect()
                }
            }
            _ => {
                if strategy.selected_market_ids.is_empty() {
                    vec![ListItem::new("No markets selected - Press [P] to pick markets")]
                } else {
                    strategy
                        .selected_market_ids
                        .iter()
                        .zip(&strategy.selected_market_names)
                        .map(|(token_id, market_name)| {
                            let truncated_id = if token_id.len() > 8 {
                                format!("{}...", &token_id[..8])
                            } else {
                                token_id.clone()
                            };
                            let truncated_name = if market_name.len() > 60 {
                                format!("{}...", &market_name[..57])
                            } else {
                                market_name.clone()
                            };
                            ListItem::new(vec![
                                Line::from(vec![
                                    Span::styled(truncated_id, Style::default().fg(Color::Cyan)),
                                    Span::raw(" - "),
                                    Span::styled(truncated_name, Style::default().fg(Color::White)),
                                ])
                            ])
                        })
                        .collect()
                }
            }
        };

        let selection_list = List::new(selection_items)
            .block(Block::default().title(selection_title).borders(Borders::ALL));

        f.render_widget(selection_list, chunks[0]);

        // Alerts/logs
        let alert_items: Vec<ListItem> = if strategy.alerts.is_empty() {
            vec![ListItem::new("No alerts")]
        } else {
            strategy
                .alerts
                .iter()
                .rev() // Show latest first
                .take(10) // Show only last 10
                .map(|alert| {
                    ListItem::new(vec![
                        Line::from(vec![
                            Span::styled(
                                alert.timestamp.format("%H:%M:%S").to_string(),
                                Style::default().fg(Color::Gray),
                            ),
                            Span::raw(" "),
                            Span::styled(
                                alert.message.clone(),
                                Style::default().fg(alert.severity.color()),
                            ),
                        ]),
                    ])
                })
                .collect()
        };

        let alerts_list = List::new(alert_items)
            .block(Block::default().title("Recent Alerts").borders(Borders::ALL));

        f.render_widget(alerts_list, chunks[1]);
    }
}
