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

    pub fn build_side(
        &self,
        levels: Vec<(String, String)>,
        descending: bool,
    ) -> Vec<PriceLevel> {
            let mut result: Vec<PriceLevel> = levels
                .into_iter()
                .filter_map(|(price, qty)| PriceLevel::new(price.to_string(), qty.to_string()))
                .filter(|level| level.has_nonzero_quantity())
                .collect();

            if descending {
                result.sort_by(|a, b| b.price.value.cmp(&a.price.value));
            } else {
                result.sort_by(|a, b| a.price.value.cmp(&b.price.value));
            }

            result.truncate(10);
            result
    }

    pub fn spread_bps(&self) -> Option<u32> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => {
                let spread = (ask.price.value - bid.price.value) / bid.price.value;
                let bps = spread * Decimal::from(10000);
                bps.to_u32()
            }
            _ => None,
        }
    }

    pub fn has_valid_checksum(&self) -> bool {
        let asks_str: String = self.asks
            .iter()
            .map(|ask| format!("{}{}", format_value(&ask.price.raw), format_value(&ask.quantity.raw)))
            .collect();
        let bids_str: String = self.bids
            .iter()
            .map(|bid| format!("{}{}", format_value(&bid.price.raw), format_value(&bid.quantity.raw)))
            .collect();
        
        let combined = format!("{}{}", asks_str, bids_str);

        let checksum = crc32fast::hash(combined.as_bytes());

        checksum == self.checksum.unwrap_or(0)
        // crc32fast::hash(combined.as_bytes()) == self.checksum.unwrap_or(0)
    }
}

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

    fn has_nonzero_quantity(&self) -> bool {
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

