use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Application configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    /// Minimum profit in basis points (1 bp = 0.01%)
    pub min_profit_bps: u32,
    /// How often to run arbitrage detection
    pub detection_interval_ms: u64,
    /// How often to refresh the UI
    pub ui_refresh_interval_ms: u64,
}

/// Order book snapshot for a single trading pair
#[derive(Debug, Clone)]
pub struct OrderBook {
    #[allow(dead_code)]
    pub symbol: String,
    pub bids: Vec<PriceLevel>, // Sorted descending by price
    pub asks: Vec<PriceLevel>, // Sorted ascending by price
    pub timestamp: DateTime<Utc>,
    pub checksum: Option<u32>, // Kraken provides checksums for validation
}

impl OrderBook {
    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids.first()
    }

    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks.first()
    }

    pub fn spread_bps(&self) -> Option<u32> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => {
                let spread = (ask.price - bid.price) / bid.price;
                Some((spread * 10000.0) as u32)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: f64,
    pub quantity: f64,
}

// /// A detected arbitrage opportunity
// #[derive(Debug, Clone)]
// pub struct ArbitrageOpportunity {
//     pub path: Vec<TradeStep>,
//     pub max_executable_usd: f64, // Maximum amount that can be traded
//     pub profit_bps: u32,         // Profit in basis points
//     pub timestamp: DateTime<Utc>,
// }

// impl ArbitrageOpportunity {
//     /// Calculate the absolute profit in USD for a given input amount
//     pub fn profit_usd(&self, input_usd: f64) -> f64 {
//         input_usd.min(self.max_executable_usd) * (self.profit_bps as f64 / 10000.0)
//     }
// }

// #[derive(Debug, Clone)]
// pub struct TradeStep {
//     pub action: TradeAction,
//     pub symbol: String,
//     pub rate: f64,
//     pub max_quantity: f64, // Maximum amount tradeable at this step
// }

// #[derive(Debug, Clone, PartialEq)]
// pub enum TradeAction {
//     Buy,
//     Sell,
// }

// impl std::fmt::Display for TradeAction {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             TradeAction::Buy => write!(f, "BUY "),
//             TradeAction::Sell => write!(f, "SELL "),
//         }
//     }
// }

/// Statistics for the application
#[derive(Debug, Clone, Default)]
pub struct Statistics {
    pub total_orderbook_updates: u64,
    pub total_opportunities_found: u64,
    pub best_opportunity_bps: u32,
    pub uptime_seconds: u64,
}

// ============================================================================
// Kraken WebSocket Protocol Types
// ============================================================================

#[derive(Serialize)]
pub struct KrakenSubscribe {
    pub method: String,
    pub params: SubscribeParams,
}

#[derive(Serialize)]
pub struct SubscribeParams {
    pub channel: String,
    pub symbol: Vec<String>,
    pub snapshot: bool,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct KrakenMessage {
    #[serde(rename = "type")]
    pub msg_type: Option<String>,
    pub channel: Option<String>,
    pub data: Option<Vec<BookData>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct BookData {
    pub symbol: String,
    pub bids: Option<Vec<BookLevel>>,
    pub asks: Option<Vec<BookLevel>>,
    pub checksum: Option<u32>,
    pub timestamp: Option<String>, // ISO 8601 timestamp
}

#[derive(Deserialize, Debug)]
pub struct BookLevel {
    pub price: f64,
    pub qty: f64,
}