use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dotenv::dotenv;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{
    io,
    time::{Duration, Instant},
    panic::AssertUnwindSafe
};
use cli_log::*;
use clap::Parser;

// Import from our local library modules
use polymarket::{App, Cli, render_ui};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    init_cli_log!();
    info!("Starting Polymarket Orderbook Viewer...");

    let cli = Cli::parse();

    // Gracefully handle panics and restore the terminal
    let result = AssertUnwindSafe(run_tui_app(cli)).await;

    // Restore terminal state
    disable_raw_mode().ok();
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).ok();

    if let Err(panic) = result {
        eprintln!("\n\nApplication panicked: {:?}\n\n", panic);
        return Err(anyhow::anyhow!("Application panicked"));
    }

    Ok(())
}


async fn run_tui_app(cli: Cli) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(cli.interval, cli.depth, &cli.private_key_env).await?;

    // Load initial data
    app.load_markets().await?;

    // If token ID is provided, use it directly
    if let Some(token_id) = cli.token_id {
        app.show_market_selector = false;
        app.show_token_selector = false;
        app.load_orderbook(&token_id).await?;
        // Start WebSocket for this specific token
        app.start_websocket_for_token(&token_id);
    }

    // Main loop
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal before returning
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = &res {
        info!("App error: {:?}", err);
    }

    res
}


async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    let tick_rate = Duration::from_millis(1); // Extremely fast polling for instant response
    let mut last_data_update = Instant::now();
    let data_update_rate = Duration::from_millis(100); // Update data 5 times per second
    let mut last_ui_update = Instant::now();
    let ui_update_rate = Duration::from_millis(1000); // Force UI update at least once per second for fading

    loop {
        let timeout = tick_rate;

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        if app.search_mode {
                            app.add_search_char('q');
                        } else {
                            return Ok(());
                        }
                    }
                    KeyCode::Char('m') => {
                        if app.search_mode {
                            app.add_search_char('m');
                        } else {
                            app.show_market_selector = true;
                            app.show_token_selector = false;
                            app.search_mode = false;
                            app.needs_redraw = true;
                        }
                    }
                    KeyCode::Char('r') => {
                        if app.search_mode {
                            app.add_search_char('r');
                        } else if let Some(ref orderbook) = app.orderbook {
                            let token_id = orderbook.token_id.clone();
                            app.load_orderbook(&token_id).await?;
                            app.needs_redraw = true;
                        }
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        if app.search_mode {
                            if matches!(key.code, KeyCode::Char('h')) {
                                app.add_search_char('h');
                            }
                        } else if !app.show_market_selector && !app.show_token_selector {
                            // Only navigate tabs when viewing orderbook
                            app.previous_tab();
                        }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if app.search_mode {
                            if matches!(key.code, KeyCode::Char('l')) {
                                app.add_search_char('l');
                            }
                        } else if !app.show_market_selector && !app.show_token_selector {
                            // Only navigate tabs when viewing orderbook
                            app.next_tab();
                        }
                    }
                    KeyCode::Char('/') => {
                        if app.search_mode {
                            app.add_search_char('/');
                        } else if app.show_market_selector {
                            app.toggle_search_mode();
                        }
                    }
                    KeyCode::Esc => {
                        if app.search_mode {
                            app.toggle_search_mode();
                        }
                    }
                    KeyCode::Up => {
                        if app.show_market_selector {
                            app.previous_market();
                        } else if app.show_token_selector {
                            app.previous_token();
                        }
                    }
                    KeyCode::Down => {
                        if app.show_market_selector {
                            app.next_market();
                        } else if app.show_token_selector {
                            app.next_token();
                        }
                    }
                    KeyCode::PageUp => {
                        if app.show_market_selector {
                            app.page_up_markets();
                        } else if app.show_token_selector {
                            app.page_up_tokens();
                        }
                    }
                    KeyCode::PageDown => {
                        if app.show_market_selector {
                            app.page_down_markets();
                        } else if app.show_token_selector {
                            app.page_down_tokens();
                        }
                    }
                    KeyCode::Enter => {
                        if app.show_market_selector {
                            app.select_market();
                        } else if app.show_token_selector {
                            app.select_token();
                            if let Some(token_id) = app.get_current_token_id() {
                                info!("Loading orderbook for token ID: {token_id}");
                                // Load initial orderbook data via API
                                app.load_orderbook(&token_id).await?;
                                // Start WebSocket for real-time updates
                                app.start_websocket_for_token(&token_id);
                                app.needs_redraw = true;
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        if app.show_token_selector {
                            app.show_market_selector = true;
                            app.show_token_selector = false;
                            app.needs_redraw = true;
                        } else if app.search_mode {
                            app.remove_search_char();
                        }
                    }
                    KeyCode::Char(ch) => {
                        if app.search_mode && !matches!(ch, 'q' | 'm' | 'r' | '/') {
                            app.add_search_char(ch);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Update price history every second
        app.update_price_history_if_needed();

        // Force UI update at least once per second for fading effects
        let force_redraw = last_ui_update.elapsed() >= ui_update_rate;

        // Redraw immediately if needed for instant feedback or if it's been a second
        if app.needs_redraw || force_redraw {
            terminal.draw(|f| render_ui(f, app))?;
            app.needs_redraw = false;
            if force_redraw {
                last_ui_update = Instant::now();
            }
        }

        // Only update data periodically, not every loop
        if last_data_update.elapsed() >= data_update_rate {
            app.update().await?;
            last_data_update = Instant::now();
        }
    }
}