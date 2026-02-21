use chrono::Utc;
use std::collections::{HashMap};
#[allow(unused_imports)]
use rust_decimal::Decimal;
#[allow(unused_imports)]
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
#[allow(unused_imports)]

use crate::types::{PriceLevel, OrderBook, Statistics};
use crate::utils::debug_log;

/// Manages all order books and ensures data accuracy by checksum
/// 
/// Design decision: We keep the full order book (up to 10 levels deep)
/// to enable accurate depth-based arbitrage calculations. We limit depth
/// to 10 levels to balance accuracy with memory usage, arbitrage opportunities
/// tend to be on the fringes of the order book anyway, so 10 is plenty.
pub struct OrderBookManager {
    books: HashMap<String, OrderBook>,
    stats: Statistics,
    start_time: chrono::DateTime<Utc>,
}

impl OrderBookManager {
    pub fn new() -> Self {
        Self {
            books: HashMap::new(),
            stats: Statistics::default(),
            start_time: Utc::now(),
        }
    }

    pub fn all_checksums_valid(&self) -> bool {
        self.books.values().all(|book| book.has_valid_checksum())
    }

    /// Update or insert an order book
    /// Kraken sends:
    /// - Initial snapshot (contains full order book)
    /// - Updates (only changed levels)
    pub fn update_book(
        &mut self,
        symbol: String,
        bids: Vec<(String, String)>,
        asks: Vec<(String, String)>,
        checksum: Option<u32>,
        is_snapshot: bool,
    ) {
        // Debug log first few updates
        static UPDATE_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let count = UPDATE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if count < 10 {
            debug_log(&format!("[update_book #{}] Symbol: '{}', Bids: {}, Asks: {}",
                count, symbol, bids.len(), asks.len()));
        }

        // Scoped block so the mutable borrow of 'book' end before we use 'self' again
        let (bid_count, ask_count) = {
            // Get existing book or create new one
            let book = self.books.entry(symbol.clone()).or_insert_with(|| OrderBook::new(symbol));
            
            if is_snapshot {
                book.load_snapshot(bids, asks);
            } else {
                book.apply_update(bids, asks);
            }
            
            // Update timestamp and checksum
            book.timestamp = Utc::now();
            book.checksum = checksum.or(book.checksum);

            (book.bids.len(), book.asks.len())
        };

        self.stats.total_orderbook_updates += 1;
        self.stats.all_checksums_valid = self.all_checksums_valid();

        if count < 10 {
            debug_log(&format!("  -> Stored. Total books: {}, This book has {} bids, {} asks",
                self.books.len(), bid_count, ask_count));
        }
    }

    /// Get order books
    pub fn get_books(&self) -> HashMap<String, OrderBook> {
        let now = Utc::now();

        // Debug: Log what we have
        static DEBUG_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let count = DEBUG_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if count < 5 {
            debug_log(&format!("[get_books #{}] Total books in storage: {}", count, self.books.len()));
            for (symbol, book) in self.books.iter() {
                let age = now.signed_duration_since(book.timestamp);
                let bid_count = book.bids.len();
                let ask_count = book.asks.len();
                debug_log(&format!("  {} - Age: {}ms, Bids: {}, Asks: {}", 
                    symbol, age.num_milliseconds(), bid_count, ask_count));
            }
        }

        let books = self.books.clone();
        
        if count < 5 {
            debug_log(&format!("  -> Returning {} books", books.len()));
        }

        books
    }

    /// Get statistics for display
    pub fn get_stats(&self) -> Statistics {
        let mut stats = self.stats.clone();
        stats.uptime_seconds = Utc::now()
            .signed_duration_since(self.start_time)
            .num_seconds() as u64;
        stats
    }

    /// Get number of active order books
    pub fn active_book_count(&self) -> usize {
        self.get_books().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_sorting() {
        let mut manager = OrderBookManager::new();

        let bids = vec![("100.0", "1.0"), ("102.0", "2.0"), ("101.0", "1.5")].iter().map(|(s, t)| (s.to_string(), t.to_string())).collect();
        let asks = vec![("105.0", "1.0"), ("103.0", "2.0"), ("104.0", "1.5")].iter().map(|(s, t)| (s.to_string(), t.to_string())).collect();

        manager.update_book("TEST/USD".to_string(), bids, asks, None, true);

        let book = manager.books.get("TEST/USD").unwrap();

        // Bids: BTreeMap is ascending internally, iterate in reverse for highest-first
        let bids: Vec<&PriceLevel> = book.bids.values().rev().collect();
        assert_eq!(bids[0].price.value, Decimal::from_str_exact("102.0").unwrap());
        assert_eq!(bids[1].price.value, Decimal::from_str_exact("101.0").unwrap());
        assert_eq!(bids[2].price.value, Decimal::from_str_exact("100.0").unwrap());

        // Asks: BTreeMap natural order is already ascending (lowest first)
        let asks: Vec<&PriceLevel> = book.asks.values().collect();
        assert_eq!(asks[0].price.value, Decimal::from_str_exact("103.0").unwrap());
        assert_eq!(asks[1].price.value, Decimal::from_str_exact("104.0").unwrap());
        assert_eq!(asks[2].price.value, Decimal::from_str_exact("105.0").unwrap());
    }

    #[test]
    fn test_checksum_verification() {
        let mut manager = OrderBookManager::new();

        // The checksum example provided at https://docs.kraken.com/api/docs/guides/spot-ws-book-v2/ should correctly parse, otherwise the checksum detection is flawed
        let bids = vec![("45283.5", "0.10000000"), ("45283.4", "1.54582015"), ("45282.1", "0.10000000"), ("45281.0", "0.10000000"), ("45280.3", "1.54592586"), ("45279.0", "0.07990000"), ("45277.6", "0.03310103"), ("45277.5", "0.30000000"), ("45277.3", "1.54602737"), ("45276.6", "0.15445238")]
            .iter().map(|(s, t)| (s.to_string(), t.to_string())).collect();
        let asks = vec![("45285.2", "0.00100000"), ("45286.4", "1.54571953"), ("45286.6", "1.54571109"), ("45289.6", "1.54560911"), ("45290.2", "0.15890660"), ("45291.8", "1.54553491"), ("45294.7", "0.04454749"), ("45296.1", "0.35380000"), ("45297.5", "0.09945542"), ("45299.5", "0.18772827")]
            .iter().map(|(s, t)| (s.to_string(), t.to_string())).collect();
        let checksum = 3310070434;

        manager.update_book("TEST/USD".to_string(), bids, asks, Some(checksum), true);

        let book = manager.books.get("TEST/USD").unwrap();

        assert!(book.has_valid_checksum());
    }
}