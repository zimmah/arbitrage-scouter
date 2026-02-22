use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::types::{ArbitrageOpportunity, Config, TradeAction, TradeStep};
use crate::orderbook::OrderBook;

#[derive(Clone)]
pub struct TriangularPath {
    base_pair: String,
    intermediate_pair: String,
    quote_pair: String,
}

/// Detects triangular arbitrage opportunities
pub struct ArbitrageDetector {
    config: Config,
    opportunities: Arc<RwLock<Vec<ArbitrageOpportunity>>>,
}

impl ArbitrageDetector {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            opportunities: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Detect all triangular arbitrage opportunities in the current order boosk
    pub fn detect_triangular_arbitrage(
        &self,
        books: &HashMap<String, OrderBook>,
    ) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();

        // Define triangular paths to check
        let paths = vec![
            TriangularPath{base_pair: "BTC/USD".to_string(), intermediate_pair: "ETH/BTC".to_string(), quote_pair: "ETH/USD".to_string()},
            TriangularPath{base_pair: "BTC/USD".to_string(), intermediate_pair: "XRP/BTC".to_string(), quote_pair: "XRP/USD".to_string()},
            TriangularPath{base_pair: "ETH/USD".to_string(), intermediate_pair: "XRP/ETH".to_string(), quote_pair: "XRP/USD".to_string()},
            TriangularPath{base_pair: "BTC/USD".to_string(), intermediate_pair: "SOL/BTC".to_string(), quote_pair: "SOL/USD".to_string()},
        ];

        for path in paths {
            // Forward: USD → base_pair → intermediate_pair → USD
            if let Some(opp) = self.check_forward_path(books, &path) {
                if opp.profit_bps >= self.config.min_profit_bps {
                    opportunities.push(opp);
                }
            }

            // Reverse: USD → intermediate_pair → base_pair → USD
            if let Some(opp) = self.check_reverse_path(books, &path) {
                if opp.profit_bps >= self.config.min_profit_bps {
                    opportunities.push(opp);
                }
            }
        }

        opportunities
    }

    /// Check forward path:  USD → base_pair → intermediate_pair → USD
    /// 
    /// Example: USD → BTC → ETH → USD
    /// 1. Buy BTC with USD (use ask price of BTC/USD)
    /// 2. Buy ETH with BTC (use ask price of ETH/BTC)
    /// 3. Sell ETH for USD (use bid price of ETH/USD)
    fn check_forward_path(
        &self,
        books: &HashMap<String, OrderBook>,
        path: &TriangularPath,
    ) -> Option<ArbitrageOpportunity> {
        let book1 = books.get(&path.base_pair)?;
        let book2 = books.get(&path.intermediate_pair)?;
        let book3 = books.get(&path.quote_pair)?;

        let (_, ask1) = book1.best_ask()?;
        let (_, ask2) = book2.best_ask()?;
        let (_, bid3) = book3.best_bid()?;

        // Calculate maximum executable amount
        // We work backwards from the final step to find the bottleneck
        
        // Step 3 bottleneck: How much quote (Example ETH) can we sell?
        let max_quote_sell = bid3.quantity.value;
        
        // Step 2 bottleneck: How much intermediate (Example BTC) do we need to get that quote (ETH)?
        let max_intermediate_for_step2 = max_quote_sell * ask2.price.value;
        let max_intermediate_step2 = ask2.quantity.value.min(max_intermediate_for_step2);
        
        // Step 1 bottleneck: How much quote (USD) to get that intermediate (BTC)?
        let max_quote_for_step1 = max_intermediate_step2 * ask1.price.value;
        let max_quote_step1 = ask1.quantity.value * ask1.price.value;
        
        // Maximum executable is the minimum of all constraints
        let max_executable_quote = max_quote_step1.min(max_quote_for_step1);
        let max_executable_quote_f64 = max_executable_quote.to_f64()?;
        
        // Calculate actual amounts at each step using max executable
        let initial_quote = max_executable_quote;
        let base_amount = initial_quote / ask1.price.value;
        let intermediate_amount = base_amount / ask2.price.value;
        let final_quote = intermediate_amount * bid3.price.value;

        // Calculate profit
        let profit = final_quote - initial_quote;
        let profit_bps = ((profit / initial_quote) * Decimal::from(10000)).to_u32()?;

        let steps = vec![
            TradeStep {
                action: TradeAction::Buy,
                symbol: path.base_pair.to_string(),
                rate: ask1.price.value.to_f64()?,
                max_quantity: base_amount.to_f64()?,
            },
            TradeStep {
                action: TradeAction::Buy,
                symbol: path.intermediate_pair.to_string(),
                rate: ask2.price.value.to_f64()?,
                max_quantity: intermediate_amount.to_f64()?,
            },
            TradeStep {
                action: TradeAction::Sell,
                symbol: path.quote_pair.to_string(),
                rate: bid3.price.value.to_f64()?,
                max_quantity: final_quote.to_f64()?,
            },
        ];

        Some(ArbitrageOpportunity {
            path: steps,
            max_executable_usd: max_executable_quote_f64,
            profit_bps,
            timestamp: Utc::now(),
        })
    }

    /// Check reverse path: USD → intermediate → base → USD
    ///
    /// Example: USD → ETH → BTC → USD
    /// 1. Buy ETH with USD (use ask price of ETH/USD)
    /// 2. Sell ETH for BTC (use bid price of ETH/BTC)
    /// 3. Sell BTC for USD (use bid price of BTC/USD)
    fn check_reverse_path(
        &self,
        books: &HashMap<String, OrderBook>,
        path: &TriangularPath,
    ) -> Option<ArbitrageOpportunity> {
        let book1 = books.get(&path.base_pair)?;
        let book2 = books.get(&path.intermediate_pair)?;
        let book3 = books.get(&path.quote_pair)?;

        let (_, bid1) = book1.best_bid()?;
        let (_, bid2) = book2.best_bid()?;
        let (_, ask3) = book3.best_ask()?;

        // Step 3 bottleneck: How much base (e.g. BTC) can we sell?
        let max_base_sell = bid1.quantity.value;

        // Step 2 bottleneck: How much intermediate (e.g. ETH) do we need to get that base?
        let max_intermediate_for_step2 = max_base_sell / bid2.price.value;
        let max_intermediate_step2 = bid2.quantity.value.min(max_intermediate_for_step2);

        // Step 1 bottleneck: How much quote (USD) to get that intermediate (ETH)?
        let max_quote_for_step1 = max_intermediate_step2 * ask3.price.value;
        let max_quote_step1 = ask3.quantity.value * ask3.price.value;

        let max_executable_quote = max_quote_step1.min(max_quote_for_step1);
        let max_executable_quote_f64 = max_executable_quote.to_f64()?;

        // Calculate actual amounts at each step
        let initial_quote = max_executable_quote;
        let intermediate_amount = initial_quote / ask3.price.value;
        let base_amount = intermediate_amount * bid2.price.value;
        let final_quote = base_amount * bid1.price.value;

        let profit = final_quote - initial_quote;
        let profit_bps = ((profit / initial_quote) * Decimal::from(10000)).to_u32()?;

        let steps = vec![
            TradeStep {
                action: TradeAction::Buy,
                symbol: path.quote_pair.to_string(),
                rate: ask3.price.value.to_f64()?,
                max_quantity: intermediate_amount.to_f64()?,
            },
            TradeStep {
                action: TradeAction::Sell,
                symbol: path.intermediate_pair.to_string(),
                rate: bid2.price.value.to_f64()?,
                max_quantity: base_amount.to_f64()?,
            },
            TradeStep {
                action: TradeAction::Sell,
                symbol: path.base_pair.to_string(),
                rate: bid1.price.value.to_f64()?,
                max_quantity: final_quote.to_f64()?,
            },
        ];

        Some(ArbitrageOpportunity {
            path: steps,
            max_executable_usd: max_executable_quote_f64,
            profit_bps,
            timestamp: Utc::now(),
        })
    }

    /// Update stored opportunities (called by detection task)
    pub async fn update_opportunities(&self, new_opportunities: Vec<ArbitrageOpportunity>) {
        let mut opps = self.opportunities.write().await;
        *opps = new_opportunities;
    }

    /// Get current opportunities for display
    pub async fn get_opportunities(&self) -> Vec<ArbitrageOpportunity> {
        self.opportunities.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::types::{CanonicalDecimal, PriceLevel};

    fn make_level(price: &str, qty: &str) -> (Decimal, PriceLevel) {
        let price = CanonicalDecimal::from_raw(price).unwrap();
        let quantity = CanonicalDecimal::from_raw(qty).unwrap();
        let key = price.value;
        (key, PriceLevel { price, quantity})
    }

    fn create_test_book(symbol: &str, bid_price: &str, ask_price: &str) -> OrderBook {
        let mut bids = BTreeMap::new();
        let mut asks = BTreeMap::new();
        let (k, v) = make_level(bid_price, "10.0");
        bids.insert(k, v);
        let (k, v) = make_level(ask_price, "10.0");
        asks.insert(k, v);
        
        OrderBook {
            symbol: symbol.to_string(),
            bids,
            asks,
            timestamp: Utc::now(),
            checksum: None,
        }
    }

    #[test]
    fn test_profitable_forward_path() {
        let config = Config {
            min_profit_bps: 10,
            detection_interval_ms: 1000,
            ui_refresh_interval_ms: 250,
        };
        let detector = ArbitrageDetector::new(config);
        let mut books = HashMap::new();

        // All books have ask > bid
        // The ETH/USD price is "too high" relative to BTC/USD * ETH/BTC,
        // creating a genuine triangular arbitrage: USD → BTC → ETH → USD
        //
        // Forward (USD → BTC → ETH → USD):
        //   1000 / 50100 = 0.01996007984 BTC
        //   0.01996007984 / 0.06010 = 0.3321144732 ETH
        //   0.3321144732 * 3050 = ~1012.95 USD  (profit)
        books.insert("BTC/USD".to_string(), create_test_book("BTC/USD", "50000.0", "50100.0"));
        books.insert("ETH/BTC".to_string(), create_test_book("ETH/BTC", "0.06000", "0.06010"));
        books.insert("ETH/USD".to_string(), create_test_book("ETH/USD", "3050.0", "3060.0"));

        let opportunities = detector.detect_triangular_arbitrage(&books);

        // Should find at least the forward path
        assert!(!opportunities.is_empty(), "Expected a profitable forward path");

        // Verify the profit is positive and above threshold
        let best = opportunities.iter().max_by_key(|o| o.profit_bps).unwrap();
        assert!(best.profit_bps >= 10, "Expected profit >= 10 bps, got {}", best.profit_bps);

        // Sanity check: verify all trade steps have sensible rates
        for step in &best.path {
            assert!(step.rate > 0.0, "Trade step rate should be positive");
            assert!(step.max_quantity > 0.0, "Trade step quantity should be positive");
        }
    }

    #[test]
    fn test_no_opportunity_when_prices_aligned() {
        let config = Config {
            min_profit_bps: 10,
            detection_interval_ms: 1000,
            ui_refresh_interval_ms: 250,
        };
        let detector = ArbitrageDetector::new(config);
        let mut books = HashMap::new();

        // Prices are internally consistent: ETH/USD mid (3000) == BTC/USD mid (50000) * ETH/BTC mid (0.06)
        // Spreads on all three pairs ensure both forward and reverse paths are lossy:
        //
        // Forward (USD → BTC → ETH → USD):
        //   1000 / 50100 = 0.01996007984 BTC
        //   0.01996007984 / 0.06010 = 0.3321144732 ETH
        //   0.3321144732 * 2990 = ~993.02 USD  (loss)
        //
        // Reverse (USD → ETH → BTC → USD):
        //   1000 / 3010 = 0.3322259136 ETH
        //   0.3322259136 * 0.06000 = 0.01993355482 BTC
        //   0.01993355482 * 49900 = ~994.68 USD  (loss)
        books.insert("BTC/USD".to_string(), create_test_book("BTC/USD", "49900.0", "50100.0"));
        books.insert("ETH/BTC".to_string(), create_test_book("ETH/BTC", "0.06000", "0.06010"));
        books.insert("ETH/USD".to_string(), create_test_book("ETH/USD", "2990.0", "3010.0"));

        let opportunities = detector.detect_triangular_arbitrage(&books);

        assert!(
            opportunities.is_empty(),
            "Expected no opportunities, but found {} with profit_bps: {:?}",
            opportunities.len(),
            opportunities.iter().map(|o| o.profit_bps).collect::<Vec<_>>()
        );
    }
}