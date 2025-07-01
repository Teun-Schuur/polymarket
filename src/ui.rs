use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Cell, Chart, Clear, Dataset, GraphType, List, ListItem, Paragraph, Row, Table, Tabs, Wrap},
    Frame,
    symbols,
};
use cli_log::{
    info, warn,
};


use crate::app::App;
use crate::data::{MarketStats, OrderBookData, SimpleOrder, OrderChangeDirection};

pub fn render_ui(f: &mut Frame, app: &mut App) {
    if app.show_market_selector || app.show_token_selector {
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
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Main content
        if app.show_market_selector {
            render_market_selector(f, app, chunks[1]);
        } else if app.show_token_selector {
            render_token_selector(f, app, chunks[1]);
        }

        // Footer
        let footer_text = if app.show_market_selector {
            if app.search_mode {
                format!("Search: {} | ↑↓: Navigate | PgUp/PgDn: Fast scroll | Enter: Select | Esc: Exit search | q: Quit", app.search_query)
            } else {
                "↑↓: Navigate | PgUp/PgDn: Fast scroll | Enter: Select | /: Search | q: Quit".to_string()
            }
        } else {
            "↑↓: Navigate | PgUp/PgDn: Fast scroll | Enter: Select | Backspace: Back to Markets | q: Quit".to_string()
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
}

fn render_market_selector(f: &mut Frame, app: &App, area: Rect) {
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

fn render_token_selector(f: &mut Frame, app: &App, area: Rect) {
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

fn render_orderbook(f: &mut Frame, app: &mut App, area: Rect) {
    if let Some(ref mut orderbook) = app.orderbook {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Combined market info and statistics header
                Constraint::Min(0),     // Orderbook and plot
            ])
            .split(area);

        // Combined market info and statistics header
        let ws_status = if app.current_websocket.is_some() {
            "🟢 Live"
        } else {
            "🔴 API Only"
        };
        render_combined_market_header(f, &orderbook.stats, orderbook, ws_status, chunks[0]);

        // Main orderbook content with plot
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Orderbook tables
                Constraint::Percentage(50), // Orderbook plot
            ])
            .split(chunks[1]);

        // Orderbook tables (left side)
        let table_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_chunks[0]);

        // Bids (left) - BUY orders
        render_order_side(f, &orderbook.bids, "Bids (BUY Orders)", Color::Green, table_chunks[0], orderbook.tick_size);
        // Asks (right) - SELL orders
        render_order_side(f, &orderbook.asks, "Asks (SELL Orders)", Color::Red, table_chunks[1], orderbook.tick_size);

        // Charts (right side) - split vertically
        // Check for Bitcoin chart before borrowing orderbook
        let market_question = orderbook.market_question.clone();
        let should_show_bitcoin = market_question.to_lowercase().contains("bitcoin") || 
                                  market_question.to_lowercase().contains("btc");
        
        let chart_chunks = if should_show_bitcoin {
            // Three charts: Bitcoin, Price history, Depth chart
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(25), // Bitcoin chart
                    Constraint::Percentage(35), // Price history chart  
                    Constraint::Percentage(40), // Orderbook depth chart
                ])
                .split(main_chunks[1])
        } else {
            // Two charts: Price history, Depth chart
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40), // Price history chart
                    Constraint::Percentage(60), // Orderbook depth chart
                ])
                .split(main_chunks[1])
        };

        if should_show_bitcoin {
            // Clone Bitcoin price data to avoid borrow checker issues
            let bitcoin_price_data = if let Some(ref btc_arc) = app.bitcoin_price {
                if let Ok(btc_data) = btc_arc.lock() {
                    // Debug log occasionally
                    if !btc_data.history.points.is_empty() && btc_data.history.points.len() % 50 == 0 {
                        info!("UI: Bitcoin price data available with {} points, current: ${:.2}", 
                              btc_data.history.points.len(), btc_data.price);
                    }
                    Some(btc_data.clone())
                } else {
                    None
                }
            } else {
                None
            };
            
            // Bitcoin chart (top)
            render_bitcoin_chart_with_data(f, bitcoin_price_data, chart_chunks[0]);
            // Price history chart (middle)
            render_price_history_chart(f, orderbook, chart_chunks[1]);
            // Orderbook depth chart (bottom)
            render_orderbook_plot(f, orderbook, chart_chunks[2]);
        } else {
            // Price history chart (top)
            render_price_history_chart(f, orderbook, chart_chunks[0]);
            // Orderbook depth chart (bottom)
            render_orderbook_plot(f, orderbook, chart_chunks[1]);
        }
    } else {
        let placeholder = Paragraph::new("Loading orderbook...")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(placeholder, area);
    }
}

fn render_order_side(
    f: &mut Frame,
    orders: &[SimpleOrder],
    title: &str,
    color: Color,
    area: Rect,
    tick_size: f64,
) {
    // Calculate decimal places based on tick size
    let decimal_places = if tick_size >= 1.0 {
        0
    } else {
        (-tick_size.log10().floor() as usize).min(6)
    };
    
    let header_cells = ["Price", "Size", "Total"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = orders.iter().map(|order| {
        let price = format!("{price:.decimal_places$}", price = order.price, decimal_places = decimal_places);
        let size = format!("{:>8.2}", order.size); // Right-aligned with width 8
        let total = format!("{:>8.2}", order.price * order.size); // Right-aligned with width 8
        
        // Determine highlight style based on change
        let row_style = if order.should_highlight() {
            match order.change_direction {
                OrderChangeDirection::Increase => Style::default().bg(Color::Green).fg(Color::Black),
                OrderChangeDirection::Decrease => Style::default().bg(Color::Red).fg(Color::White),
                OrderChangeDirection::None => Style::default(),
            }
        } else {
            Style::default()
        };
        
        Row::new(vec![
            Cell::from(price).style(row_style),
            Cell::from(size).style(row_style),
            Cell::from(total).style(row_style),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ]
    )
        .header(header)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::default().fg(color).add_modifier(Modifier::BOLD)))
        .column_spacing(1);

    f.render_widget(table, area);
}

fn render_market_stats(f: &mut Frame, stats: &MarketStats, orderbook: &OrderBookData, ws_status: &str, area: Rect) {
    // Calculate decimal places based on tick size
    let decimal_places = if orderbook.tick_size >= 1.0 {
        0
    } else {
        (-orderbook.tick_size.log10().floor() as usize).min(6)
    };

    
    let stats_lines = [
        format!("Spread: {spread:.decimal_places$} | Tick Size: {tick_size:.decimal_places$}  |  Last Updated: {last_updated}  |  Data Source: {ws_status}", 
                spread = stats.spread, 
                tick_size = orderbook.tick_size, 
                last_updated = orderbook.last_updated.format("%H:%M:%S UTC"),
                ws_status = ws_status,
                decimal_places = decimal_places),
    ];
    
    let stats_text = stats_lines.join("\n");
    
    let stats_para = Paragraph::new(stats_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(Block::default()
            .borders(Borders::ALL));
    
    f.render_widget(stats_para, area);
}

fn render_orderbook_plot(f: &mut Frame, orderbook: &mut OrderBookData, area: Rect) {
    let bids = &orderbook.bids;
    let asks = &orderbook.asks;

    let best_bid = bids.first().map(|b| b.price).unwrap_or(0.5);
    let best_ask = asks.first().map(|a| a.price).unwrap_or(0.5);

    let ticks_around_spread = 20;

    let (min_tick, max_tick) = if best_bid > 0.0 && best_ask > 0.0 && best_ask > best_bid {
        let mid_price = (best_bid + best_ask) / 2.0;
        let mid_tick = (mid_price / orderbook.tick_size).round() as i64;
        let half_range = ticks_around_spread / 2;
        let start_tick = (mid_tick - half_range as i64).max(0);
        let end_tick = mid_tick + half_range as i64;
        let max_valid_tick = (1.0 / orderbook.tick_size).floor() as i64;
        let constrained_end_tick = end_tick.min(max_valid_tick);
        orderbook.chart_center_price = Some(mid_tick as f64 * orderbook.tick_size);
        orderbook.chart_needs_recentering = false;
        (start_tick, constrained_end_tick)
    } else {
        let all_prices: Vec<f64> = bids.iter().chain(asks.iter()).map(|o| o.price).filter(|&p| (0.0..=1.0).contains(&p)).collect();
        if all_prices.is_empty() { return; }
        let min_order_price = all_prices.iter().copied().fold(f64::INFINITY, f64::min);
        let max_order_price = all_prices.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min_tick = ((min_order_price / orderbook.tick_size).floor() as i64 - 10).max(0);
        let max_valid_tick = (1.0 / orderbook.tick_size).floor() as i64;
        let max_tick = ((max_order_price / orderbook.tick_size).ceil() as i64 + 10).min(max_valid_tick);
        let mid_tick = (min_tick + max_tick) / 2;
        orderbook.chart_center_price = Some(mid_tick as f64 * orderbook.tick_size);
        orderbook.chart_needs_recentering = false;
        (min_tick, max_tick)
    };

    if max_tick <= min_tick { return; }

    let min_price = min_tick as f64 * orderbook.tick_size;
    let num_ticks = (max_tick - min_tick) as usize;
    let mut bid_depths = vec![0.0; num_ticks];
    let mut ask_depths = vec![0.0; num_ticks];

    // Build bid depths (cumulative from highest price down)
    let mut cumulative_bid_size = 0.0;
    let mut sorted_bids = bids.to_vec();
    sorted_bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
    for bid in sorted_bids.iter() {
        if !(0.0..=1.0).contains(&bid.price) { continue; }
        cumulative_bid_size += bid.size;
        let tick_index = ((bid.price - min_price) / orderbook.tick_size).round() as usize;
        if tick_index < num_ticks {
            bid_depths[tick_index] = cumulative_bid_size;
        }
    }
    // Fill gaps in bid depths (propagate cumulative sizes down)
    for i in (0..num_ticks - 1).rev() {
        if bid_depths[i] < bid_depths[i + 1] {
            bid_depths[i] = bid_depths[i + 1];
        }
    }

    // Build ask depths (cumulative from lowest price up)
    let mut cumulative_ask_size = 0.0;
    let mut sorted_asks = asks.to_vec();
    sorted_asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));
    for ask in sorted_asks.iter() {
        if !(0.0..=1.0).contains(&ask.price) { continue; }
        cumulative_ask_size += ask.size;
        let tick_index = ((ask.price - min_price) / orderbook.tick_size).round() as usize;
        if tick_index < num_ticks {
            ask_depths[tick_index] = cumulative_ask_size;
        }
    }
    // Fill gaps in ask depths (propagate cumulative sizes up)
    for i in 1..num_ticks {
        if ask_depths[i] < ask_depths[i - 1] {
            ask_depths[i] = ask_depths[i - 1];
        }
    }

    // Create data points, but only where orders actually exist
    let bid_data: Vec<(f64, f64)> = bid_depths.iter().enumerate()
        .filter_map(|(i, &depth)| {
            let price = (min_tick as f64 + i as f64) * orderbook.tick_size;
            // Only include bid data points at or below the best bid
            if depth > 0.0 && price <= best_bid {
                Some((price, depth))
            } else {
                None
            }
        })
        .collect();

    let ask_data: Vec<(f64, f64)> = ask_depths.iter().enumerate()
        .filter_map(|(i, &depth)| {
            let price = (min_tick as f64 + i as f64) * orderbook.tick_size;
            // Only include ask data points at or above the best ask
            if depth > 0.0 && price >= best_ask {
                Some((price, depth))
            } else {
                None
            }
        })
        .collect();

    if bid_data.is_empty() && ask_data.is_empty() { return; }

    let max_depth = bid_data.iter().chain(ask_data.iter())
        .map(|(_, depth)| *depth)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(1.0);

    if max_depth <= 0.0 { return; }

    let mut datasets = Vec::new();
    
    if !bid_data.is_empty() {
        datasets.push(Dataset::default()
            .name("Bids")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Green))
            .graph_type(GraphType::Line)
            .data(&bid_data));
    }
    
    if !ask_data.is_empty() {
        datasets.push(Dataset::default()
            .name("Asks")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Red))
            .graph_type(GraphType::Line)
            .data(&ask_data));
    }

    let min_price_display = min_tick as f64 * orderbook.tick_size;
    let max_price_display = max_tick as f64 * orderbook.tick_size;

    let decimal_places = if orderbook.tick_size >= 1.0 { 0 } else { (-orderbook.tick_size.log10().floor() as usize).min(6) };

    let chart = Chart::new(datasets)
        .block(Block::default().title(format!("Orderbook Depth - Spread: {:.4}", best_ask - best_bid)).borders(Borders::ALL))
        .x_axis(
            Axis::default()
                .title("Price")
                .style(Style::default().fg(Color::Gray))
                .bounds([min_price_display, max_price_display])
                .labels(vec![
                    Span::from(format!("{:.decimal_places$}", min_price_display, decimal_places = decimal_places)),
                    Span::from(format!("{:.decimal_places$}", (min_price_display + max_price_display) / 2.0, decimal_places = decimal_places)),
                    Span::from(format!("{:.decimal_places$}", max_price_display, decimal_places = decimal_places)),
                ]),
        )
        .y_axis(
            Axis::default()
                .title("Depth")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, max_depth])
                .labels(vec![
                    Span::from("0"),
                    Span::from(format!("{:.0}", max_depth / 2.0)),
                    Span::from(format!("{:.0}", max_depth)),
                ]),
        );
    f.render_widget(chart, area);
}

fn render_price_history_chart(f: &mut Frame, orderbook: &OrderBookData, area: Rect) {
    let price_points: Vec<(f64, f64)> = orderbook.price_history.points.iter()
        .map(|p| (p.timestamp.timestamp() as f64, p.price))
        .collect();

    if price_points.len() < 2 {
        let no_data = Paragraph::new("Collecting price data...").style(Style::default().fg(Color::Gray)).alignment(Alignment::Center);
        f.render_widget(Block::default().title("Price History").borders(Borders::ALL), area);
        f.render_widget(no_data, area);
        return;
    }

    let (min_time, max_time) = orderbook.price_history.get_time_range().unwrap();
    let (min_price, max_price) = orderbook.price_history.get_price_range().unwrap();

    let datasets = vec![Dataset::default()
        .name("Price")
        .marker(symbols::Marker::Dot)
        .style(Style::default().fg(Color::Cyan))
        .graph_type(GraphType::Line)
        .data(&price_points)];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(format!("Price History - Current: {:.4}", orderbook.price_history.current_price().unwrap_or(orderbook.stats.mid_price)))
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Time")
                .style(Style::default().fg(Color::Gray))
                .bounds([min_time.timestamp() as f64, max_time.timestamp() as f64])
                .labels(vec![
                    Span::from(min_time.format("%H:%M:%S").to_string()),
                    Span::from(max_time.format("%H:%M:%S").to_string()),
                ]),
        )
        .y_axis(
            Axis::default()
                .title("Price")
                .style(Style::default().fg(Color::Gray))
                .bounds([min_price, max_price])
                .labels(vec![
                    Span::from(format!("{:.4}", min_price)),
                    Span::from(format!("{:.4}", (min_price + max_price) / 2.0)),
                    Span::from(format!("{:.4}", max_price)),
                ]),
        );
    f.render_widget(chart, area);
}

fn render_bitcoin_chart_with_data(f: &mut Frame, bitcoin_price_data: Option<crate::data::BitcoinPrice>, area: Rect) {
    if bitcoin_price_data.is_none() {
        let block = Block::default().borders(Borders::ALL).title("Bitcoin Price - Connecting...");
        f.render_widget(block, area);
        let placeholder = Paragraph::new("Connecting to Bitcoin price feed...").style(Style::default().fg(Color::Yellow)).alignment(Alignment::Center);
        f.render_widget(placeholder, area);
        return;
    }

    let btc_data = bitcoin_price_data.as_ref().unwrap();
    let price_points: Vec<(f64, f64)> = btc_data.history.points.iter()
        .map(|p| (p.timestamp.timestamp() as f64, p.price))
        .collect();

    if price_points.len() < 2 {
        let no_data = Paragraph::new("Collecting Bitcoin price data...").style(Style::default().fg(Color::Gray)).alignment(Alignment::Center);
        f.render_widget(Block::default().title("Bitcoin Price").borders(Borders::ALL), area);
        f.render_widget(no_data, area);
        return;
    }

    let (min_time, max_time) = btc_data.history.get_time_range().unwrap();
    let (min_price, max_price) = btc_data.history.get_price_range().unwrap();

    let datasets = vec![Dataset::default()
        .name("BTC Price")
        .marker(symbols::Marker::Dot)
        .style(Style::default().fg(Color::Rgb(255, 165, 0))) // Bitcoin orange
        .graph_type(GraphType::Line)
        .data(&price_points)];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(format!("Bitcoin Price - Current: ${:.2}", btc_data.price))
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Time")
                .style(Style::default().fg(Color::Gray))
                .bounds([min_time.timestamp() as f64, max_time.timestamp() as f64])
                .labels(vec![
                    Span::from(min_time.format("%H:%M:%S").to_string()),
                    Span::from(max_time.format("%H:%M:%S").to_string()),
                ]),
        )
        .y_axis(
            Axis::default()
                .title("Price ($)")
                .style(Style::default().fg(Color::Gray))
                .bounds([min_price, max_price])
                .labels(vec![
                    Span::from(format!("${:.2}", min_price)),
                    Span::from(format!("${:.2}", (min_price + max_price) / 2.0)),
                    Span::from(format!("${:.2}", max_price)),
                ]),
        );
    f.render_widget(chart, area);
}

fn render_tab_bar(f: &mut Frame, app: &App, area: Rect) {
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

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
fn render_market_price_history(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref price_history) = app.market_price_history {
        let market_name = if let Some(ref orderbook) = app.orderbook {
            &orderbook.market_question
        } else {
            "Market Price History"
        };
        
        // Convert price history data to chart dataset
        let mut chart_data = Vec::new();
        for point in price_history.history.iter() {
            chart_data.push((point.t as f64, point.p));
        }
        
        if chart_data.is_empty() {
            let message = Paragraph::new("No price history data available")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Price History - {}", market_name)));
            f.render_widget(message, area);
            return;
        }
        
        // Get tick size from orderbook, default to 0.001 if not available
        let tick_size = app.orderbook.as_ref().map(|ob| ob.tick_size).unwrap_or(0.001);
        
        // Calculate decimal places based on tick size
        let decimal_places = if tick_size >= 1.0 {
            0
        } else {
            (-tick_size.log10().floor() as usize).min(6)
        };
        
        let y_min = 0.0;
        let y_max = 1.0;
        
        // Generate more y-axis labels rounded to tick size
        let num_ticks = 6; // Number of tick marks
        let mut y_labels = Vec::new();
        for i in 0..num_ticks {
            let value = y_min + (y_max - y_min) * (i as f64) / (num_ticks - 1) as f64;
            // Round to nearest tick size
            let rounded_value = (value / tick_size).round() * tick_size;
            y_labels.push(Span::styled(
                format!("{:.decimal_places$}", rounded_value, decimal_places = decimal_places),
                Style::default().fg(Color::Gray)
            ));
        }

        // Get time range for x-axis
        let x_min = chart_data.first().map(|(t, _)| *t).unwrap_or(0.0);
        let x_max = chart_data.last().map(|(t, _)| *t).unwrap_or(1.0);
        
        // Create time labels from Unix timestamps with intelligent formatting
        let time_span_seconds = x_max - x_min;
        let _start_time = chrono::DateTime::from_timestamp(x_min as i64, 0)
            .unwrap_or_else(|| chrono::Utc::now());
        let _end_time = chrono::DateTime::from_timestamp(x_max as i64, 0)
            .unwrap_or_else(|| chrono::Utc::now());
        
        // Determine number of x-axis labels based on available width (estimate)
        let available_width = area.width.saturating_sub(10); // Account for borders and margins
        let max_labels = (available_width / 12).max(2).min(8) as usize; // Each label needs ~12 chars width
        
        // Generate x-axis labels with appropriate intervals
        let mut x_labels = Vec::new();
        for i in 0..max_labels {
            let t = x_min + (x_max - x_min) * (i as f64) / (max_labels - 1) as f64;
            let datetime = chrono::DateTime::from_timestamp(t as i64, 0)
                .unwrap_or_else(|| chrono::Utc::now());
            
            let label = if time_span_seconds < 3600.0 {
                // Less than 1 hour: show minutes and seconds
                datetime.format("%M:%S").to_string()
            } else if time_span_seconds < 86400.0 {
                // Less than 1 day: show hours and minutes
                datetime.format("%H:%M").to_string()
            } else if time_span_seconds < 2592000.0 {
                // Less than 30 days: show month/day and hour
                datetime.format("%m/%d %H:00").to_string()
            } else if time_span_seconds < 31536000.0 {
                // Less than 1 year: show month/day
                datetime.format("%m/%d").to_string()
            } else {
                // More than 1 year: show year/month
                datetime.format("%Y/%m").to_string()
            };
            
            x_labels.push(Span::styled(label, Style::default().fg(Color::Gray)));
        }
        
        let dataset = Dataset::default()
            .name("Price")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&chart_data);
        
        let chart = Chart::new(vec![dataset])
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("Price History - {market_name}")))
            .x_axis(Axis::default()
                .title("Time")
                .style(Style::default().fg(Color::Gray))
                .bounds([x_min, x_max])
                .labels(x_labels))
            .y_axis(Axis::default()
                .title("Price")
                .style(Style::default().fg(Color::Gray))
                .bounds([y_min, y_max])
                .labels(y_labels));
        
        f.render_widget(chart, area);
    } else {
        let message = if app.orderbook.is_some() {
            "Loading price history..."
        } else {
            "No market selected"
        };
        
        let loading = Paragraph::new(message)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Price History"));
        f.render_widget(loading, area);
    }
}

fn render_combined_market_header(f: &mut Frame, stats: &MarketStats, orderbook: &OrderBookData, ws_status: &str, area: Rect) {
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
        spread = stats.spread,
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