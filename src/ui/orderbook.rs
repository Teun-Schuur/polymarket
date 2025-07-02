use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use cli_log::info;

use crate::app::App;
use crate::data::{MarketStats, OrderBookData, SimpleOrder, OrderChangeDirection};
use super::{charts::{render_orderbook_plot, render_price_history_chart, render_bitcoin_chart_with_data}, components::render_combined_market_header};

pub fn render_orderbook(f: &mut Frame, app: &mut App, area: Rect) {
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
                match btc_arc.lock() {
                    Ok(btc_data) => {
                        // Debug log occasionally
                        if !btc_data.history.points.is_empty() && btc_data.history.points.len() % 50 == 0 {
                            info!("UI: Bitcoin price data available with {} points, current: ${:.2}", 
                                  btc_data.history.points.len(), btc_data.price);
                        }
                        Some(btc_data.clone())
                    }
                    Err(_) => None
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

pub fn render_order_side(
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

pub fn render_market_stats(f: &mut Frame, stats: &MarketStats, orderbook: &OrderBookData, ws_status: &str, area: Rect) {
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
