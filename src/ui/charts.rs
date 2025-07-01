use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Tabs},
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
                    Span::from(format!("{:.4}", min_price)),
                    Span::from(format!("{:.4}", (min_price + max_price) / 2.0)),
                    Span::from(format!("{:.4}", max_price)),
                ]),
        );
    f.render_widget(chart, area);
}

pub fn render_bitcoin_chart_with_data(f: &mut Frame, bitcoin_price_data: Option<crate::data::BitcoinPrice>, area: Rect) {
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
        let _time_span_seconds = x_max - x_min;
        let start_time = chrono::DateTime::from_timestamp(x_min as i64, 0)
            .unwrap_or_else(|| chrono::Utc::now());
        let end_time = chrono::DateTime::from_timestamp(x_max as i64, 0)
            .unwrap_or_else(|| chrono::Utc::now());

        let datasets = vec![Dataset::default()
            .name("Price")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::Cyan))
            .graph_type(GraphType::Line)
            .data(&chart_data)];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(format!("Price History - {}", market_name))
                    .borders(Borders::ALL),
            )
            .x_axis(
                Axis::default()
                    .title("Time")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([x_min, x_max])
                    .labels(vec![
                        Span::from(start_time.format("%m/%d %H:%M").to_string()),
                        Span::from(end_time.format("%m/%d %H:%M").to_string()),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .title("Price")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([y_min, y_max])
                    .labels(y_labels),
            );
        f.render_widget(chart, area);
    } else {
        let message = Paragraph::new("Loading price history...")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Price History"));
        f.render_widget(message, area);
    }
}
