use cli_log::warn;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
    Frame,
    symbols,
};

use crate::app::App;
use crate::data::OrderBookData;

pub fn render_orderbook_plot(f: &mut Frame, orderbook: &mut OrderBookData, area: Rect) {
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
        warn!("Invalid orderbook state: best_bid: {best_bid}, best_ask: {best_ask}");
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

    if max_tick <= min_tick { 
        warn!("Invalid tick range: min_tick: {min_tick}, max_tick: {max_tick}");
        return; 
    }

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

    // Create block-style data points
    let half_tick = orderbook.tick_size / 2.0;
    
    let mut bid_data: Vec<(f64, f64)> = Vec::new();
    for (i, &depth) in bid_depths.iter().enumerate() {
        let price = (min_tick as f64 + i as f64) * orderbook.tick_size;
        // Only include bid data points at or below the best bid
        if depth > 0.0 && price <= best_bid {
            // For bids: go from tick center to left edge (bid width extends left)
            bid_data.push((price - half_tick, depth));
            bid_data.push((price, depth));
        }
    }
    // Add line from best bid to 0 at the spread
    if !bid_data.is_empty() {
        bid_data.push((best_bid, bid_data.last().unwrap().1));
        bid_data.push((best_bid, 0.0));
    }

    let mut ask_data: Vec<(f64, f64)> = Vec::new();
    for (i, &depth) in ask_depths.iter().enumerate() {
        let price = (min_tick as f64 + i as f64) * orderbook.tick_size;
        // Only include ask data points at or above the best ask
        if depth > 0.0 && price >= best_ask {
            // For asks: go from tick center to right edge (ask width extends right)
            ask_data.push((price, depth));
            ask_data.push((price + half_tick, depth));
        }
    }
    // Add line from best ask to 0 at the spread
    if !ask_data.is_empty() {
        ask_data.insert(0, (best_ask, 0.0));
        ask_data.insert(1, (best_ask, ask_data[2].1));
    }

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
                    Span::from(format!("{min_price_display:.decimal_places$}")),
                    Span::from(format!("{:.decimal_places$}", (min_price_display + max_price_display) / 2.0, decimal_places = decimal_places)),
                    Span::from(format!("{max_price_display:.decimal_places$}")),
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
                    Span::from(format!("{max_depth:.0}")),
                ]),
        );
    f.render_widget(chart, area);
}

pub fn render_price_history_chart(f: &mut Frame, orderbook: &OrderBookData, area: Rect) {
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
                    Span::from(format!("{min_price:.4}")),
                    Span::from(format!("{:.4}", (min_price + max_price) / 2.0)),
                    Span::from(format!("{max_price:.4}")),
                ]),
        );
    f.render_widget(chart, area);
}

pub fn render_crypto_chart_with_data(
    f: &mut Frame, 
    crypto_data: Option<crate::data::CryptoPrice>, 
    symbol: &crate::websocket::CryptoSymbol,
    area: Rect
) {
    let (name, color, symbol_str) = match symbol {
        crate::websocket::CryptoSymbol::Bitcoin => ("Bitcoin", Color::Rgb(255, 165, 0), "BTC"), // Orange
        crate::websocket::CryptoSymbol::Ethereum => ("Ethereum", Color::Rgb(98, 126, 234), "ETH"), // Blue
        crate::websocket::CryptoSymbol::Solana => ("Solana", Color::Rgb(138, 255, 255), "SOL"), // Cyan
    };
    
    if crypto_data.is_none() {
        let block = Block::default().borders(Borders::ALL).title(format!("{name} Price - Connecting..."));
        f.render_widget(block, area);
        let placeholder = Paragraph::new(format!("Connecting to {name} price feed..."))
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        f.render_widget(placeholder, area);
        return;
    }

    let crypto = crypto_data.as_ref().unwrap();
    let price_points: Vec<(f64, f64)> = crypto.history.points.iter()
        .map(|p| (p.timestamp.timestamp() as f64, p.price))
        .collect();

    if price_points.len() < 2 {
        let no_data = Paragraph::new(format!("Collecting {name} price data..."))
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(Block::default().title(format!("{name} Price")).borders(Borders::ALL), area);
        f.render_widget(no_data, area);
        return;
    }

    let (min_time, max_time) = crypto.history.get_time_range().unwrap();
    let (min_price, max_price) = crypto.history.get_price_range().unwrap();

    let datasets = vec![Dataset::default()
        .name(format!("{symbol_str} Price"))
        .marker(symbols::Marker::Dot)
        .style(Style::default().fg(color))
        .graph_type(GraphType::Line)
        .data(&price_points)];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(format!("{} Price - Current: ${:.2}", name, crypto.price))
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
                    Span::from(format!("${min_price:.2}")),
                    Span::from(format!("${:.2}", (min_price + max_price) / 2.0)),
                    Span::from(format!("${max_price:.2}")),
                ]),
        );
    f.render_widget(chart, area);
}

// Backward compatibility
pub fn render_bitcoin_chart_with_data(f: &mut Frame, bitcoin_price_data: Option<crate::data::BitcoinPrice>, area: Rect) {
    render_crypto_chart_with_data(f, bitcoin_price_data, &crate::websocket::CryptoSymbol::Bitcoin, area);
}

pub fn render_market_price_history(f: &mut Frame, app: &App, area: Rect) {
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
            let no_data = Paragraph::new("No price history data available")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            f.render_widget(Block::default().title("Market Price History").borders(Borders::ALL), area);
            f.render_widget(no_data, area);
            return;
        }

        // Calculate price range
        let prices: Vec<f64> = chart_data.iter().map(|(_, price)| *price).collect();
        let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        // Calculate time range and convert to dates
        let times: Vec<f64> = chart_data.iter().map(|(time, _)| *time).collect();
        let min_time = times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_time = times.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        // Convert timestamps to DateTime for formatting
        // let min_date = chrono::DateTime::from_timestamp(min_time as i64, 0).unwrap_or_default();
        // let max_date = chrono::DateTime::from_timestamp(max_time as i64, 0).unwrap_or_default();
        const NUM_DATES: u32 = 5;
        let mut dates: Vec<Span> = vec![];
        for i in 0..NUM_DATES {
            let fraction = i as f64 / (NUM_DATES - 1) as f64;
            let timestamp = min_time + fraction * (max_time - min_time);
            if let Some(date) = chrono::DateTime::from_timestamp(timestamp as i64, 0) {
                let formatted_date = date.format("%d/%m %H:%M").to_string();
                dates.push(Span::from(formatted_date));
            } else {
                warn!("Invalid timestamp: {timestamp}");
            }
        }


        let datasets = vec![Dataset::default()
            .name("Market Price")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::Green))
            .graph_type(GraphType::Line)
            .data(&chart_data)];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(format!("Market Price History - {market_name}"))
                    .borders(Borders::ALL),
            )
            .x_axis(
                Axis::default()
                    .title("Time")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([min_time, max_time])
                    .labels(dates.clone())
            )
            .y_axis(
                Axis::default()
                    .title("Price")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([min_price, max_price])
                    .labels(vec![
                        Span::from(format!("{min_price:.3}")),
                        Span::from(format!("{:.3}", (min_price + max_price) / 2.0)),
                        Span::from(format!("{max_price:.3}")),
                    ]),
            );
        f.render_widget(chart, area);
    } else {
        let no_data = Paragraph::new("Loading market price history...")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        f.render_widget(Block::default().title("Market Price History").borders(Borders::ALL), area);
        f.render_widget(no_data, area);
    }
}
