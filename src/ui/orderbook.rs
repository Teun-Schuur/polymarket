#![allow(unused_imports)]

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use cli_log::*;

use crate::app::App;
use crate::data::{MarketStats, OrderBookData, SimpleOrder, OrderChangeDirection};
use super::{charts::{render_orderbook_plot, render_price_history_chart, render_crypto_chart_with_data}, components::render_combined_market_header};
use crate::websocket::CryptoSymbol;

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
        // Check for crypto charts before borrowing orderbook
        let market_question = orderbook.market_question.clone();
        let question_lower = market_question.to_lowercase();
        
        let mut relevant_cryptos = Vec::new();
        if question_lower.contains("bitcoin") || market_question.contains("BTC") {
            relevant_cryptos.push(CryptoSymbol::Bitcoin);
        }
        if question_lower.contains("ethereum") || market_question.contains("ETH") {
            relevant_cryptos.push(CryptoSymbol::Ethereum);
        }
        if question_lower.contains("solana") || market_question.contains("SOL") {
            relevant_cryptos.push(CryptoSymbol::Solana);
        }
        
        let crypto_count = relevant_cryptos.len();
        
        let chart_chunks = if crypto_count > 0 {
            // Multiple charts: Crypto charts + Price history + Depth chart
            let mut constraints = Vec::new();
            
            // Each crypto chart gets equal space at the top
            for _ in 0..crypto_count {
                constraints.push(Constraint::Percentage(20));
            }
            
            // Price history and orderbook split the remaining space
            let remaining = 100 - (crypto_count * 20) as u16;
            constraints.push(Constraint::Percentage(remaining / 2)); // Price history
            constraints.push(Constraint::Percentage(remaining / 2)); // Orderbook depth
            
            Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(main_chunks[1])
        } else {
            // Two charts: Price history, Depth chart
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(50), // Price history chart
                    Constraint::Percentage(50), // Orderbook depth chart
                ])
                .split(main_chunks[1])
        };

        // Render crypto charts
        for (i, crypto_symbol) in relevant_cryptos.iter().enumerate() {
            let crypto_price_data = app.crypto_prices.get(crypto_symbol)
                .and_then(|arc| arc.lock().ok())
                .map(|data| data.clone());
            
            render_crypto_chart_with_data(f, crypto_price_data, crypto_symbol, chart_chunks[i]);
        }
        
        let price_history_idx = crypto_count;
        let orderbook_idx = crypto_count + 1;
        
        // Price history chart
        render_price_history_chart(f, orderbook, chart_chunks[price_history_idx]);
        // Orderbook depth chart
        render_orderbook_plot(f, orderbook, chart_chunks[orderbook_idx]);
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
