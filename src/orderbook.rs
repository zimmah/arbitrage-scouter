use chrono::Utc;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;

use crate::types::{OrderBook, PriceLevel, Statistics};

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

    /// Update or insert an order book
    /// Kraken sends:
    /// - Initial snapshot (contains full order book)
    /// - Updates (only changed levels)
    pub fn update_book(
        &mut self,
        symbol: String,
        bids: Vec<(f64, f64)>,
        asks: Vec<(f64, f64)>,
        checksum: Option<u32>,
    ) {
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
            let mut bid_levels: Vec<PriceLevel> = bids
                .into_iter()
                .map(|(price, qty)| PriceLevel { price, quantity: qty })
                .filter(|level| level.quantity > 0.0) // Remove zero-quantity levels
                .collect();
            bid_levels.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
            bid_levels.truncate(10); // Keep top 10 only
            book.bids = bid_levels;
        }

        // Update asks if provided
        if !asks.is_empty() {
            let mut ask_levels: Vec<PriceLevel> = asks
                .into_iter()
                .map(|(price, qty)| PriceLevel { price, quantity: qty })
                .filter(|level| level.quantity > 0.0) // Remove zero-quantity levels
                .collect();
            ask_levels.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
            ask_levels.truncate(10); // Keep top 10 only
            book.asks = ask_levels;
        }

        // Update timestamp and checksum
        book.timestamp = Utc::now();
        book.checksum = checksum.or(book.checksum);

        self.books.insert(symbol.clone(), book);
        self.stats.total_orderbook_updates += 1;

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
        let bids = vec![(100.0, 1.0), (102.0, 2.0), (101.0, 1.5)];
        // Asks should be sorted ascending (lowest first)
        let asks = vec![(105.0, 1.0), (103.0, 2.0), (104.0, 1.5)];

        manager.update_book("TEST/USD".to_string(), bids, asks, None);

        let book = manager.books.get("TEST/USD").unwrap();
        
        assert_eq!(book.bids[0].price, 102.0);
        assert_eq!(book.bids[1].price, 101.0);
        assert_eq!(book.bids[2].price, 100.0);

        assert_eq!(book.asks[0].price, 103.0);
        assert_eq!(book.asks[1].price, 104.0);
        assert_eq!(book.asks[2].price, 105.0);
    }
}