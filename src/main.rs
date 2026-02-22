mod orderbook;
mod types;
mod ui;
mod websocket;
mod utils;
mod arbitrage;

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::orderbook::OrderBookManager;
use crate::websocket::run_websocket_client;
use crate::ui::run_tui;
use crate::types::Config;
use crate::arbitrage::ArbitrageDetector;

#[tokio::main]
async fn main() -> Result<()> {
    // Configuration
    let config = Config {
        min_profit_bps: 10, // 0.10% = 10 basis points
        detection_interval_ms: 1000,
        ui_refresh_interval_ms: 250,
    };

    // Trading pairs to monitor
    // These form triangular arbitrage paths
    let symbols = vec![
        "BTC/USD", "ETH/USD", "ETH/BTC",
        "XRP/USD", "XRP/BTC", "XRP/ETH",
        "SOL/USD", "SOL/BTC",
    ];

    let (manager, resync_rx) = OrderBookManager::new();
    // Shared state: order books and detected opportunities
    let orderbook_manager = Arc::new(RwLock::new(manager));
    let arbitrage_detector = Arc::new(ArbitrageDetector::new(config.clone()));

    // Spawn WebSocket task
    let ws_handle = tokio::spawn({
        let orderbook_manager = Arc::clone(&orderbook_manager);
        async move {
            run_websocket_client(symbols, orderbook_manager, resync_rx).await
        }
    });

    // Wait for initial data to populate
    eprintln!("\nWaiting 5 seconds for initial data...");
    eprintln!("(Debug info will be written to debug.log)");
    tokio::time::sleep(Duration::from_secs(5)).await;

    eprintln!("Starting TUI...\n");
    // Small delay to let messages finish printing
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Spawn arbitrage detection task
    let detection_handle = tokio::spawn({
        let orderbook_manager = Arc::clone(&orderbook_manager);
        let arbitrage_detector = Arc::clone(&arbitrage_detector);
        let interval_ms = config.detection_interval_ms;

        async move {
            let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
            loop {
                interval.tick().await;

                // Get current order books
                let books = {
                    let manager = orderbook_manager.read().await;
                    manager.get_books()
                };

                // Detect opportunities
                let opportunities = arbitrage_detector.detect_triangular_arbitrage(&books);

                if !opportunities.is_empty() {
                    let best_bps = opportunities.iter().map(|o| o.profit_bps).max().unwrap_or(0);
                    let mut manager = orderbook_manager.write().await;
                    manager.record_opportunities(opportunities.len(), best_bps);
                }
                // update detector's stored opportunities
                arbitrage_detector.update_opportunities(opportunities).await;
            }
        }
    });

    // Run the TUI (blocks until user exits)
    let tui_result = run_tui(
        Arc::clone(&orderbook_manager),
        Arc::clone(&arbitrage_detector),
        config.ui_refresh_interval_ms,
    ).await;

    // Cleanup: abort background tasks
    ws_handle.abort();
    detection_handle.abort();

    tui_result
}
