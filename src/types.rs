use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use chrono::{DateTime, Utc};

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Minimum profit in basis points (1 bp = 0.01%)
    pub min_profit_bps: u32,
    /// How often to run arbitrage detection
    pub detection_interval_ms: u64,
    /// How often to refresh the UI
    pub ui_refresh_interval_ms: u64,
}

/// A detected arbitrage opportunity
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ArbitrageOpportunity {
    pub path: Vec<TradeStep>,
    pub max_executable_usd: f64, // Maximum amount that can be traded
    pub profit_bps: u32,         // Profit in basis points
    pub timestamp: DateTime<Utc>,
}

// impl ArbitrageOpportunity {
//     /// Calculate the absolute profit in USD for a given input amount
//     pub fn profit_usd(&self, input_usd: f64) -> f64 {
//         input_usd.min(self.max_executable_usd) * (self.profit_bps as f64 / 10000.0)
//     }
// }

#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: CanonicalDecimal,
    pub quantity: CanonicalDecimal,
}

impl PriceLevel {
    pub fn new(price_str: String, quantity_str: String) -> Option<Self> {
        Some(Self { 
            price: CanonicalDecimal::from_raw(&price_str)?,
            quantity: CanonicalDecimal::from_raw(&quantity_str)?,
        })
    }

    pub fn has_nonzero_quantity(&self) -> bool {
        self.quantity.value > Decimal::ZERO
    }
}

#[derive(Debug, Clone)]
pub struct CanonicalDecimal {
    pub value: Decimal,
    pub raw: String,
}

impl CanonicalDecimal {
    pub fn from_raw(raw_json: &str) -> Option<Self> {
        let value = Decimal::from_str_exact(raw_json).ok()?;
        Some(Self { value, raw: raw_json.to_owned() })
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TradeStep {
    pub action: TradeAction,
    pub symbol: String,
    pub rate: f64,
    pub max_quantity: f64, // Maximum amount tradeable at this step
}

#[derive(Debug, Clone, PartialEq)]
pub enum TradeAction {
    Buy,
    Sell,
}

impl std::fmt::Display for TradeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradeAction::Buy => write!(f, "BUY "),
            TradeAction::Sell => write!(f, "SELL "),
        }
    }
}

/// Statistics for the application
#[derive(Debug, Clone, Default)]
pub struct Statistics {
    pub total_orderbook_updates: u64,
    pub total_opportunities_found: u64,
    pub best_opportunity_bps: u32,
    pub uptime_seconds: u64,
    pub all_checksums_valid: bool,
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
#[allow(dead_code)] // channel isn't currently used, but it's good to know it exists
pub struct KrakenMessage {
    #[serde(rename = "type")]
    pub msg_type: Option<String>,
    pub channel: Option<String>,
    pub data: Option<Vec<BookData>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // timestamp isn't currently used, but it's good to know it's there
pub struct BookData {
    pub symbol: String,
    pub bids: Option<Vec<BookLevel>>,
    pub asks: Option<Vec<BookLevel>>,
    pub checksum: Option<u32>,
    pub timestamp: Option<String>, // ISO 8601 timestamp
}

#[derive(Deserialize, Debug)]
pub struct BookLevel {
    pub price:  Box<RawValue>,
    pub qty: Box<RawValue>,
}

