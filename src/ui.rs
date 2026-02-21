use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

// use crate::arbitrage::ArbitrageDetector; todo
use crate::orderbook::OrderBookManager;

// run the terminal UI
pub async fn run_tui(
    orderbook_manager: Arc<RwLock<OrderBookManager>>,
    // arbitrage_detector: Arc<ArbitrageDetector>,
    ui_refresh_interval_ms: u64,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Clear the terminal to avoid artifacts from debug output
    terminal.clear()?;

    // Run the UI loop
    let result = run_ui_loop(&mut terminal, orderbook_manager, /* arbitrage_detector, */ ui_refresh_interval_ms).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_ui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    orderbook_manager: Arc<RwLock<OrderBookManager>>,
    // arbitrage_detector: Arc<ArbitrageDetector>,
    ui_refresh_interval_ms: u64,
) -> Result<()> {
    loop {
        // Get current data
        let (books, stats) = {
            let manager = orderbook_manager.read().await;
            (manager.get_books(), manager.get_stats())
        };
        // let opportunities = arbitrage_detector.get_opportunities().await;

        // Render UI
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(10), // Order Books
                    Constraint::Min(15), // Opportunities
                    Constraint::Length(6), // Stats
                ])
                .split(f.area());

            // Header
            render_header(f, chunks[0], &stats);

            // Order books
            render_orderbooks(f, chunks[1], &books);

            // Opportunities
            render_opportunities(f, chunks[2], /* &opportunities */); // render placeholder for now

            // Statistics
            render_stats(f, chunks[3], &stats);
        })?;

        // Check for exit key (non-blocking)
        if event::poll(Duration::from_millis(ui_refresh_interval_ms))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        return Ok(());
                    }
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
    }
}

fn render_header(
    f: &mut ratatui::Frame,
    area: Rect,
    stats: &crate::types::Statistics,
) {
    let uptime_str = format_uptime(stats.uptime_seconds);
    let text = vec![Line::from(vec![
        Span::styled("Kraken ", Style::default().fg(Color::Rgb(116, 52, 243)).add_modifier(Modifier::BOLD)),
        Span::styled("Arbitrage Scouter", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled("Press 'q' to quit", Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
        Span::styled(format!("Uptime: {}", uptime_str), Style::default().fg(Color::Green)),
    ])];

    let paragraph = Paragraph::new(text).block(Block::default().borders(Borders::ALL));
    f.render_widget(paragraph, area);
}

fn render_orderbooks(
    f: &mut ratatui::Frame,
    area: Rect,
    books: &std::collections::HashMap<String, crate::orderbook::OrderBook>,
) {
    let mut items: Vec<ListItem> = Vec::new();

    if books.is_empty() {
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                "Waiting for order book data from WebSocket...",
                Style::default().fg(Color::Yellow),
            ),
        ])));
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                "(Check debug.log for debug messages",
                Style::default().fg(Color::DarkGray),
            ),
        ])));
    } else {
        let mut sorted_books: Vec<_> = books.iter().collect();
        sorted_books.sort_by_key(|(symbol, _)| symbol.as_str());

        for (symbol, book) in sorted_books {
            if let (Some((_, bid)), Some((_, ask))) = (book.best_bid(), book.best_ask()) {
                let spread_bps = book.spread_bps().unwrap_or(0.0);

                let line = Line::from(vec![
                    Span::styled(format!("{:12}", symbol), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::raw("  Bid: "),
                    Span::styled(format!("{:>12}", bid.price.value), Style::default().fg(Color::Green)),
                    Span::raw("  Ask: "),
                    Span::styled(format!("{:>12}", ask.price.value), Style::default().fg(Color::Red)),
                    Span::raw("  Spread: "),
                    Span::styled(format!("{:>4.2} bps", spread_bps), Style::default().fg(Color::Yellow)),
                ]);
                items.push(ListItem::new(line));
            }
        }
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Order Books (Live)"));
    f.render_widget(list, area);
}

// Placeholder
fn render_opportunities(
    f: &mut ratatui::Frame,
    area: Rect,
    // opportunities: &[crate::types::ArbitrageOpportunity],
) {
    let mut items: Vec<ListItem> = Vec::new();

    items.push(ListItem::new(Line::from(Span::styled(
        "No arbitrage opportunities detected",
        Style::default().fg(Color::DarkGray),
    ))));

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Arbitrage Opportunities"));
    f.render_widget(list, area);
}

fn render_stats(
    f: &mut ratatui::Frame,
    area: Rect,
    stats: &crate::types::Statistics,
) {
    let checksum_color = if stats.all_checksums_valid { Color::Green } else { Color::Red };
    let checksum_text = if stats.all_checksums_valid { "✅ All valid" } else { "❌ Invalid" };

    let text = vec![
        Line::from(vec![
            Span::styled("Order Book Updates: ", Style::default().fg(Color::Gray)),
            Span::styled(stats.total_orderbook_updates.to_string(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Opportunities Found: ", Style::default().fg(Color::Gray)),
            Span::styled(stats.total_opportunities_found.to_string(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Best Opportunity: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.2}%", stats.best_opportunity_bps as f64 / 100.0),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("Valid Checksums: ", Style::default().fg(Color::Gray)),
            Span::styled(checksum_text, Style::default().fg(checksum_color)),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Statistics"));
    f.render_widget(paragraph, area);
}

fn format_uptime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}