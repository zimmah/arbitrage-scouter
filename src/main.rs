mod orderbook;
mod types;
mod ui;
mod websocket;
mod utils;

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::orderbook::OrderBookManager;
use crate::websocket::run_websocket_client;
use crate::ui::run_tui;
use crate::types::Config;

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
    // todo: arbitrage detector
    // let arbitrage_detector = Arc etc

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

    // todo
    // Spawn arbitrage detection task
    // let detection_handle = etc...

    // Run the TUI (blocks until user exits)
    let tui_result = run_tui(
        Arc::clone(&orderbook_manager),
        // Arc::clone(&arbitrage_detector),
        config.ui_refresh_interval_ms,
    ).await;

    // Cleanup: abort background tasks
    ws_handle.abort();
    // detection_handle.abort(); // uncomment once decection handle is implemented

    tui_result
}
