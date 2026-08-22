use chrono::Utc;
use kraken_ws_v2::OrderBook;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::{HashMap, HashSet};

use crate::types::Statistics;

/// Best-bid to best-ask spread in basis points, for display.
pub fn spread_bps(book: &OrderBook) -> Option<f32> {
    let bid = book.best_bid()?.price();
    let ask = book.best_ask()?.price();
    let bps = (ask - bid) / bid * Decimal::from(10000);
    bps.to_f32()
}

/// Holds the latest validated order books and app-level statistics.
///
/// Book maintenance itself (snapshots, incremental updates, CRC32 checksum
/// validation, resync) lives in the kraken-ws-v2 crate; every book stored
/// here has already passed validation. This manager only tracks which
/// symbols are mid-resync so the UI can report checksum health.
pub struct OrderBookManager {
    books: HashMap<String, OrderBook>,
    resyncing: HashSet<String>,
    stats: Statistics,
    start_time: chrono::DateTime<Utc>,
}

impl OrderBookManager {
    pub fn new() -> Self {
        Self {
            books: HashMap::new(),
            resyncing: HashSet::new(),
            stats: Statistics::default(),
            start_time: Utc::now(),
        }
    }

    /// Stores a validated book delivered by the client.
    pub fn update_book(&mut self, book: OrderBook) {
        self.resyncing.remove(book.symbol());
        self.books.insert(book.symbol().to_owned(), book);
        self.stats.total_orderbook_updates += 1;
    }

    /// Marks a symbol as resyncing after a checksum failure; cleared when
    /// the next validated book arrives.
    pub fn mark_resyncing(&mut self, symbol: String) {
        self.books.remove(&symbol);
        self.resyncing.insert(symbol);
    }

    pub fn all_checksums_valid(&self) -> bool {
        self.resyncing.is_empty()
    }

    /// Get order books
    pub fn get_books(&self) -> HashMap<String, OrderBook> {
        self.books.clone()
    }

    /// Get statistics for display
    pub fn get_stats(&self) -> Statistics {
        let mut stats = self.stats.clone();
        stats.uptime_seconds = Utc::now()
            .signed_duration_since(self.start_time)
            .num_seconds() as u64;
        stats.all_checksums_valid = self.all_checksums_valid();

        stats
    }

    /// Record arbitrage opportunities so they can be displayed in the UI
    pub fn record_opportunities(&mut self, count: usize, best_bps: u32) {
        self.stats.total_opportunities_found += count as u64;
        if best_bps > self.stats.best_opportunity_bps {
            self.stats.best_opportunity_bps = best_bps;
        }
    }

    /// Get number of active order books
    pub fn active_book_count(&self) -> usize {
        self.books.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_ws_v2::{BookKind, BookMessage, Level};

    fn book_with_top(symbol: &str, bid: &str, ask: &str) -> OrderBook {
        let mut book = OrderBook::new(symbol, 10);
        let msg = BookMessage::new(
            symbol,
            BookKind::Snapshot,
            vec![Level::parse(bid, "1.0").unwrap()],
            vec![Level::parse(ask, "1.0").unwrap()],
            None,
        );
        book.apply(&msg).unwrap();
        book
    }

    #[test]
    fn stores_books_and_tracks_updates() {
        let mut manager = OrderBookManager::new();
        manager.update_book(book_with_top("BTC/USD", "100.0", "100.1"));
        manager.update_book(book_with_top("BTC/USD", "100.2", "100.3"));

        assert_eq!(manager.active_book_count(), 1);
        assert_eq!(manager.get_stats().total_orderbook_updates, 2);
        let books = manager.get_books();
        let best_bid = books["BTC/USD"].best_bid().unwrap().price();
        assert_eq!(best_bid.to_string(), "100.2");
    }

    #[test]
    fn resync_state_drives_checksum_health() {
        let mut manager = OrderBookManager::new();
        manager.update_book(book_with_top("BTC/USD", "100.0", "100.1"));
        assert!(manager.all_checksums_valid());

        manager.mark_resyncing("BTC/USD".to_string());
        assert!(!manager.all_checksums_valid());
        assert_eq!(manager.active_book_count(), 0);

        manager.update_book(book_with_top("BTC/USD", "100.0", "100.1"));
        assert!(manager.all_checksums_valid());
    }

    #[test]
    fn spread_is_reported_in_bps() {
        let book = book_with_top("TEST/USD", "10000.0", "10001.0");
        let bps = spread_bps(&book).unwrap();
        assert!((bps - 1.0).abs() < 0.01, "expected ~1 bps, got {bps}");
    }
}
