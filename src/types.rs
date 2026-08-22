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
    pub max_executable_usd: f64,  // Maximum amount that can be traded
    pub profit_bps: u32,          // Profit in basis points
    pub timestamp: DateTime<Utc>, // currently unused, therefore #[allow(dead_code)]
}

impl ArbitrageOpportunity {
    /// Calculate the absolute profit in USD for a given input amount
    pub fn profit_usd(&self, input_usd: f64) -> f64 {
        input_usd.min(self.max_executable_usd) * (self.profit_bps as f64 / 10000.0)
    }
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

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TradeStep {
    pub action: TradeAction,
    pub symbol: String,
    pub rate: f64,
    pub max_quantity: f64, // Maximum amount tradeable at this step
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
