use anyhow::Result;
use kraken_ws_v2::{Event, KrakenClient};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::orderbook::OrderBookManager;
use crate::utils::debug_log;

/// Feeds validated market data from the kraken-ws-v2 client into the
/// order book manager.
///
/// Connection management, reconnection with backoff, resubscription, and
/// checksum validation with automatic resync all live in the crate; this
/// task only routes its events into application state.
pub async fn run_websocket_client(
    symbols: Vec<&str>,
    orderbook_manager: Arc<RwLock<OrderBookManager>>,
) -> Result<()> {
    debug_log(&format!("[WebSocket] Subscribing to symbols: {symbols:?}"));

    let mut client = KrakenClient::connect().await?;
    client.subscribe_book(symbols).await?;

    while let Some(event) = client.recv().await {
        match event {
            Event::Book(update) => {
                orderbook_manager.write().await.update_book(update.book);
            }
            Event::BookResync { symbol } => {
                debug_log(&format!(
                    "[WebSocket] checksum mismatch, resyncing {symbol}"
                ));
                orderbook_manager.write().await.mark_resyncing(symbol);
            }
            Event::Connected => debug_log("[WebSocket] connected"),
            Event::Disconnected { reason } => {
                debug_log(&format!(
                    "[WebSocket] disconnected ({reason}), reconnecting"
                ));
            }
            Event::Subscribed { channel, symbol } => {
                debug_log(&format!("[WebSocket] subscribed to {channel}:{symbol}"));
            }
            Event::SubscriptionFailed {
                channel,
                symbol,
                message,
            } => {
                debug_log(&format!(
                    "[WebSocket] subscription to {channel}:{symbol} rejected: {message}"
                ));
            }
            _ => {}
        }
    }

    Ok(())
}
