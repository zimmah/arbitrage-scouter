use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::orderbook::OrderBookManager;
use crate::types::{BookData, KrakenMessage, KrakenSubscribe, SubscribeParams};
use crate::utils::debug_log;

/// Run the WebSocket client with automatic reconnection
/// 
/// Design decisions:
/// - Automatic reconnection on disconnect (with backoff)
/// - No authentication needed for public market data
/// - Uses tokio-tungstenite for native async/await support
pub async fn run_websocket_client(
    symbols: Vec<&str>,
    orderbook_manager: Arc<RwLock<OrderBookManager>>,
    mut resync_rx: tokio::sync::mpsc::Receiver<String>,
) -> Result<()> {
    let symbols: Vec<String> = symbols.iter().map(|s| s.to_string()).collect();
    let url = "wss://ws.kraken.com/v2";

    let base_delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(60);
    let mut current_delay = base_delay;

    debug_log(&format!("[WebSocket] Starting connection to {}", url));
    debug_log(&format!("[WebSocket] Subscribing to symbols: {:?}", symbols));

    loop {
        let connected_at = tokio::time::Instant::now();

        match connect_and_subscribe(url, &symbols, &orderbook_manager, &mut resync_rx).await {
            Ok(_) => {
                debug_log(&format!("[WebSocket] Connection closed normally"));
            }
            Err(e) => {
                debug_log(&format!("[WebSocket] Error: {:?}", e));
            }
        }

        // Reset backoff if connection was stable for >30s
        if connected_at.elapsed() > Duration::from_secs(30) {
            current_delay = base_delay;
        }

        debug_log(&format!("[WebSocket] Reconnecting in {:?}...", current_delay));
        tokio::time::sleep(current_delay).await;

        // Exponential backoff with cap
        current_delay = (current_delay * 2).min(max_delay);
    }
}

async fn connect_and_subscribe(
    url: &str,
    symbols: &[String],
    orderbook_manager: &Arc<RwLock<OrderBookManager>>,
    resync_rx: &mut tokio::sync::mpsc::Receiver<String>,
) -> Result<()> {
    // Connect to WebSocket
    let (ws_stream, response) = connect_async(url)
        .await
        .context("Failed to connect to Kraken WebSocket")?;
    
    debug_log(&format!("[WebSocket] Connected! Response: {}", response.status()));

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to order book channel
    let subscribe_msg = KrakenSubscribe {
        method: "subscribe".to_string(),
        params: SubscribeParams {
            channel: "book".to_string(),
            symbol: symbols.to_vec(),
            snapshot: true,
        },
    };

    let subscribe_json = serde_json::to_string(&subscribe_msg)?;
    debug_log(&format!("[WebSocket] Sending subscription: {}", subscribe_json));
    write.send(Message::Text(subscribe_json)).await?;

    let mut message_count = 0;

    // Process incoming messages
    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        message_count += 1;
                        if message_count <= 3 {
                            debug_log(&format!("WebSocket Message #{}: {}", message_count, &text[..text.len().min(300)]));
                        }
                        handle_message(&text, orderbook_manager, message_count).await?;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        write.send(Message::Pong(data)).await?;
                    }
                    Some(Ok(Message::Close(_))) => {
                        debug_log("[WebSocket] Server closed connection");
                        break;
                    }
                    Some(Err(e)) => return Err(e.into()),
                    None => break,
                    _ => {}
                }
            }
            Some(symbol) = resync_rx.recv() => {
                debug_log(&format!("[WebSocket] Checksum invalid, resyncing {}", symbol));
                let resub = serde_json::json!({
                    "method": "subscribe",
                    "params": {
                        "channel": "book",
                        "symbol": [symbol],
                        "snapshot": true
                    }
                });
                write.send(Message::Text(resub.to_string())).await?;
            }
        }
    }


    Ok(())
}

async fn handle_message(
    text: &str,
    orderbook_manager: &Arc<RwLock<OrderBookManager>>,
    message_number: usize,
) -> Result<()> {
    // Parse Kraken message
    let kraken_msg: KrakenMessage = match serde_json::from_str(text) {
        Ok(msg) => msg,
        Err(e) => {
            // Check if it's a subscription response
            if text.contains("\"method\":\"subscribe\"") && text.contains("\"success\":true") {
                if message_number <= 10 {
                    debug_log(&format!("[WebSocket] ✓ Subscription confirmed"));
                }
                return Ok(());
            }
            // Check if it's a status update
            if text.contains("\"channel\":\"status\"") {
                debug_log(&format!("[Websocket] Status update: {}", &text[..text.len().min(200)]));
                return Ok(());
            }
            // Check if it's a heartbeat
            if text.contains("heartbeat") {
                return Ok(());
            }
            // Unknown message type
            debug_log(&format!("[WebSocket] Failed to parse message: {}", e));
            debug_log(&format!("[WebSocket] Message was: {}", &text[..text.len().min(200)]));
            return Ok(());
        }
    };

    // Process order book data
    if let Some(data) = kraken_msg.data {
        let is_snapshot = kraken_msg.msg_type.as_deref() == Some("snapshot");

        if message_number <= 10 {
            debug_log(&format!("[WebSocket] Processing {} order book updates", data.len()));
        }

        for book_data in data {
            let bid_count = book_data.bids.as_ref().map(|b| b.len()).unwrap_or(0);
            let ask_count = book_data.asks.as_ref().map(|b| b.len()).unwrap_or(0);

            if message_number <= 10 {
                debug_log(&format!("[WebSocket]   {} - {} bids, {} asks", book_data.symbol, bid_count, ask_count))
            }

            update_orderbook(orderbook_manager, book_data, is_snapshot).await;
        }

        // Log stats after first few messages
        if message_number == 10 {
            let manager = orderbook_manager.read().await;
            let book_count = manager.active_book_count();
            debug_log(&format!("[WebSocket] Status: {} active order books", book_count));
            debug_log(&format!("[WebSocket] (Further debug messages will be surpressed)"));
        }
    }

    Ok(())
}

async fn update_orderbook(
    orderbook_manager: &Arc<RwLock<OrderBookManager>>,
    book_data: BookData,
    is_snapshot: bool,
) {
    let bids = book_data.bids.unwrap_or_default()
        .into_iter().map(|l| (l.price.get().to_owned(), l.qty.get().to_owned())).collect();
    let asks = book_data.asks.unwrap_or_default()
        .into_iter().map(|l| (l.price.get().to_owned(), l.qty.get().to_owned())).collect();

    let mut manager = orderbook_manager.write().await;
    manager.update_book(book_data.symbol, bids, asks, book_data.checksum, is_snapshot);
}