use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::utils::format_value;

fn deserialize_number_as_raw_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    // Capture the raw JSON value - for numbers this preserves the exact text
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()), // serde::json::Number preserves the original representation
        _ => Err(serde::de::Error::custom("expected string or number")),
    }
}

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

// enum BookState {
//     Uninitialized,
//     Synced,
// }

#[derive(Debug, Clone)]
pub struct CanonicalDecimal {
    pub value: Decimal,
    pub raw: String,
}

impl CanonicalDecimal {
    pub fn new(raw: String) -> Option<Self> {
        let value = Decimal::from_str_exact(&raw).ok()?;
        Some(Self { value, raw })
    }
}

/// Order book snapshot for a single trading pair
#[derive(Debug, Clone)]
pub struct OrderBook {
    #[allow(dead_code)]
    pub symbol: String,
    pub bids: BTreeMap<Decimal, PriceLevel>, // ascending by key, iterate in reverse for descending
    pub asks: BTreeMap<Decimal, PriceLevel>, // ascending by key
    pub timestamp: DateTime<Utc>,
    pub checksum: Option<u32>, // Kraken provides checksums for validation
}

impl OrderBook {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            timestamp: Utc::now(),
            checksum: None,
        }
    } 

    pub fn best_bid(&self) -> Option<(&Decimal, &PriceLevel)> {
        self.bids.iter().next_back() // highest bid
    }

    pub fn best_ask(&self) -> Option<(&Decimal, &PriceLevel)> {
        self.asks.iter().next() // lowest ask
    }
    
    pub fn spread_bps(&self) -> Option<u32> {
        let (bid_price, _) = self.best_bid()?;
        let (ask_price, _) = self.best_ask()?;
        let spread = (ask_price - bid_price) / bid_price;
        let bps = spread * Decimal::from(10000);
        bps.to_u32()
    }

    pub fn load_snapshot(
        &mut self,
        bids: Vec<(String, String)>,
        asks: Vec<(String, String)>,
    ) {
        self.bids.clear();
        self.asks.clear();

        for (price, qty) in bids {
            self.apply_bid_delta(price, qty);
        }

        for (price, qty) in asks {
            self.apply_ask_delta(price, qty);
        }

        self.truncate_to_depth();
    }

    pub fn apply_update(
        &mut self,
        bids: Vec<(String, String)>,
        asks: Vec<(String, String)>,
    ) {
        for (price, qty) in bids {
            self.apply_bid_delta(price, qty);
        }
        for (price, qty) in asks {
            self.apply_ask_delta(price, qty);
        }

        self.truncate_to_depth();
    }

    pub fn apply_bid_delta(&mut self, price: String, qty: String) {
        if let Some(level) = PriceLevel::new(price, qty) {
            if level.has_nonzero_quantity() {
                self.bids.insert(level.price.value, level);
            } else {
                self.bids.remove(&level.price.value);
            }
        }
    }

    pub fn apply_ask_delta(&mut self, price: String, qty: String) {
        if let Some(level) = PriceLevel::new(price, qty) {
            if level.has_nonzero_quantity() {
                self.asks.insert(level.price.value, level);
            } else {
                self.asks.remove(&level.price.value);
            }
        }
    }

    pub fn truncate_to_depth(&mut self) {
        let depth = 10; // magic number is fine here

        // Bids: BTreeMap is ascending, so lowest bids are at the front — remove those (they're worst)
        while self.bids.len() > depth {
            let worst = *self.bids.keys().next().unwrap();
            self.bids.remove(&worst);
        }

        // Asks: BTreeMap is ascending, so highest asks are at the back — remove those (they're worst)
        while self.asks.len() > depth {
            let worst = *self.asks.keys().next_back().unwrap();
            self.asks.remove(&worst);
        }
    }

    pub fn has_valid_checksum(&self) -> bool {
        let asks_str: String = self.asks
            .values()
            .map(|level| format!("{}{}", format_value(&level.price.raw), format_value(&level.quantity.raw)))
            .collect();

        let bids_str: String = self.bids
            .values()
            .rev() // order in reverse because best bids are the highest
            .map(|level| format!("{}{}", format_value(&level.price.raw), format_value(&level.quantity.raw)))
            .collect();
        
        let combined = format!("{}{}", asks_str, bids_str);
        crc32fast::hash(combined.as_bytes()) == self.checksum.unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: CanonicalDecimal,
    pub quantity: CanonicalDecimal,
}

impl PriceLevel {
    pub fn new(price_str: String, quantity_str: String) -> Option<Self> {
        Some(Self { 
            price: CanonicalDecimal::new(price_str)?,
            quantity: CanonicalDecimal::new(quantity_str)?,
        })
    }

    pub fn has_nonzero_quantity(&self) -> bool {
        self.quantity.value > Decimal::ZERO
    }
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
    #[serde(deserialize_with = "deserialize_number_as_raw_string")]
    pub price:  String,
    #[serde(deserialize_with = "deserialize_number_as_raw_string")]
    pub qty: String,
}

