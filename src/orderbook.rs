use chrono::Utc;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;

#[allow(unused_imports)]
use rust_decimal::Decimal;
#[allow(unused_imports)]
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};

use crate::types::{OrderBook, Statistics};

fn debug_log(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug.log")
    {
        let _ = writeln!(file, "{}", msg);
    }
}

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
    pub fn update_book<S: AsRef<str>>(
        &mut self,
        symbol: String,
        bids: Vec<(S, S)>,
        asks: Vec<(S, S)>,
        checksum: Option<u32>,
    ) {
        let bids: Vec<(String, String)> = bids.into_iter().map(|(p, q)| (p.as_ref().to_string(), q.as_ref().to_string())).collect();
        let asks: Vec<(String, String)> = asks.into_iter().map(|(p, q)| (p.as_ref().to_string(), q.as_ref().to_string())).collect();
        // Debug first few updates
        static UPDATE_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let count = UPDATE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if count < 10 {
            debug_log(&format!("[update_book #{}] Symbol: '{}', Bids: {}, Asks: {}",
                count, symbol, bids.len(), asks.len()));
        }

        // Get existing book or create new one
        let mut book = self.books.get(&symbol).cloned().unwrap_or_else(|| OrderBook {
            symbol: symbol.clone(),
            bids: Vec::new(),
            asks: Vec::new(),
            timestamp: Utc::now(),
            checksum: None,
        });

        // Update bids if provided
        if !bids.is_empty() {
            book.bids = book.build_side(bids, true);
        }

        // Update asks if provided
        if !asks.is_empty() {
            book.asks = book.build_side(asks, false);
        }

        // Update timestamp and checksum
        book.timestamp = Utc::now();
        book.checksum = checksum.or(book.checksum);

        self.books.insert(symbol.clone(), book);
        self.stats.total_orderbook_updates += 1;
        self.stats.all_checksums_valid = self.all_checksums_valid();

        if count < 10 {
            let stored_book = self.books.get(&symbol).unwrap();
            debug_log(&format!("  -> Stored. Total books: {}, This book has {} bids, {} asks",
                self.books.len(), stored_book.bids.len(), stored_book.asks.len()));
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

        // Bids should be sorted descending (highest first)
        let bids = vec![("100.0", "1.0"), ("102.0", "2.0"), ("101.0", "1.5")];
        // Asks should be sorted ascending (lowest first)
        let asks = vec![("105.0", "1.0"), ("103.0", "2.0"), ("104.0", "1.5")];

        manager.update_book("TEST/USD".to_string(), bids, asks, None);

        let book = manager.books.get("TEST/USD").unwrap();
        
        assert_eq!(book.bids[0].price.value, Decimal::from_f32(102.0).expect("should be 102.0"));
        assert_eq!(book.bids[1].price.value, Decimal::from_f32(101.0).expect("should be 101.0"));
        assert_eq!(book.bids[2].price.value, Decimal::from_f32(100.0).expect("should be 100.0"));

        assert_eq!(book.asks[0].price.value, Decimal::from_f32(103.0).expect("should be 103.0"));
        assert_eq!(book.asks[1].price.value, Decimal::from_f32(104.0).expect("should be 104.0"));
        assert_eq!(book.asks[2].price.value, Decimal::from_f32(105.0).expect("should be 105.0"));
    }

    #[test]
    fn test_checksum_verification() {
        let mut manager = OrderBookManager::new();

        // The checksum example provided at https://docs.kraken.com/api/docs/guides/spot-ws-book-v2/ should correctly parse, otherwise the checksum detection is flawed
        let bids = vec![("45283.5", "0.10000000"), ("45283.4", "1.54582015"), ("45282.1", "0.10000000"), ("45281.0", "0.10000000"), ("45280.3", "1.54592586"), ("45279.0", "0.07990000"), ("45277.6", "0.03310103"), ("45277.5", "0.30000000"), ("45277.3", "1.54602737"), ("45276.6", "0.15445238")];
        let asks = vec![("45285.2", "0.00100000"), ("45286.4", "1.54571953"), ("45286.6", "1.54571109"), ("45289.6", "1.54560911"), ("45290.2", "0.15890660"), ("45291.8", "1.54553491"), ("45294.7", "0.04454749"), ("45296.1", "0.35380000"), ("45297.5", "0.09945542"), ("45299.5", "0.18772827")];
        let checksum = 3310070434;

        manager.update_book("TEST/USD".to_string(), bids, asks, Some(checksum));

        let book = manager.books.get("TEST/USD").unwrap();

        assert!(book.has_valid_checksum());
    }
}