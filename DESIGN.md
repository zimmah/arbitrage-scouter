# Design Document

This document covers the technical decisions, architectural tradeoffs, and async patterns used in this project.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Data Flow](#data-flow)
3. [Performance Considerations](#performance-considerations)
4. [Depth Calculation Design](#depth-calculation-design)
5. [Production Awareness & Real-World Constraints](#production-awareness--real-world-constraints)
6. [UI Design](#ui-design)
7. [Testing Strategy](#testing-strategy)

---

## Architecture Overview

### Module Responsibilities

```
main.rs          - Task orchestration and shared state ownership
types.rs         - Data structures; no business logic
orderbook.rs     - Order book state management
arbitrage.rs     - Detection algorithms
websocket.rs     - Network I/O and protocol handling
ui.rs            - Terminal rendering and user input
```

**Principle**: Each module has a single, clear responsibility. There are no circular dependencies.

### Why This Structure?

- **Testability**: Pure functions in `arbitrage.rs` can be unit tested without any I/O
- **Clarity**: Each module can be understood in isolation
- **Maintainability**: UI changes do not affect arbitrage logic, and vice versa
- **Reusability**: `arbitrage.rs` could be reused with an entirely different frontend

---

## Data Flow

### WebSocket → OrderBook → Arbitrage → UI

```
1. Kraken WebSocket sends an order book update
   ↓
2. websocket.rs parses the JSON payload
   ↓
3. OrderBookManager.update_book() acquires a write lock
   ↓
4. ArbitrageDetector reads all books every 1s (read lock)
   ↓
5. ArbitrageDetector calculates opportunities
   ↓
6. ArbitrageDetector updates its stored results (write lock)
   ↓
7. UI reads books and opportunities (read locks)
   ↓
8. UI renders to the terminal
```

**Key design principle**: The UI never communicates with the WebSocket client directly. Each component depends only on its immediate downstream.

---

## Performance Considerations

### Memory Usage

| Component | Estimate |
|---|---|
| Order books (8 pairs × 2 sides × 10 levels × ~32 bytes) | ~5 KB |
| Opportunities (Vec, typically < 10 items × ~200 bytes) | < 2 KB |
| Total (including Tokio runtime) | < 10 MB |

Memory usage is negligible for this application.

### CPU Usage
The arbitrage detector runs once per second, checking approximately 8 triangular paths with roughly 10 floating-point operations each. Despite using rust_decimal for precision (which is more expensive than native floats), measured CPU usage remains well under 1% on modern hardware at this interval. If parallelism were ever needed, path checking could be distributed across threads using rayon, though this is unnecessary at the current scale.

### Network

WebSocket traffic from Kraken is approximately 1–5 KB/s of compressed JSON. Latency is not a concern since this system does not execute trades.

### Lock Contention

`RwLock` is used throughout to optimise for the read-heavy access pattern:

- **Read locks** never contend with each other (multiple concurrent readers are permitted)
- **Write locks** are acquired on each market data update (typically ~277 times/second under normal activity, higher during volatile periods), and once per second by the detector

Total write lock hold time is well under 0.1% of wall time, so lock contention is not a bottleneck at this scale.

At higher scale, per-symbol lock sharding or a lock-free snapshot approach (e.g. `DashMap`, or atomic pointer swapping) would be worth evaluating. For this project, a single `RwLock` over the full book map is simple, correct, and sufficient.

### Order Book Cloning

`get_books()` clones the entire `HashMap` on each detection interval. This is the correct approach for this architecture: it gives the detector a stable snapshot to work on without holding the read lock during calculations (which would delay writers), and without the lifetime complexity of passing references across lock boundaries. The memory overhead is negligible at this scale. In a production system with thousands of symbols, an atomic pointer swap to an immutable snapshot would avoid the allocation cost while preserving the same semantics.

### Debug Logging

`debug_log()` uses blocking file I/O (`OpenOptions`) inside an async context. This is acceptable for a development tool but would be inappropriate in production, where a non-blocking structured logger such as `tracing` with a dedicated logging task would be used instead.

---

## Depth Calculation Design

### The Problem

A naive implementation uses a fixed notional amount:

```rust
let initial_amount = 1000.0; // Arbitrary constant "magic number"
```

This is problematic because the result depends entirely on the chosen constant. If available liquidity is $100, the profit calculation is overstated. If liquidity is $10,000, the opportunity is understated.

### The Solution

The system instead calculates the maximum executable amount by working backward through the trade path:

```rust
// Work backwards from the final step to find the liquidity bottleneck
let max_quote_sell = bid3.quantity;
let max_intermediate_for_step2 = max_quote_sell * ask2.price;
let max_executable_usd = /* minimum constraint across all steps */;
```

This approach respects actual liquidity at each step, identifies the binding constraint, and produces a realistic upper bound on profit. The tradeoff is added code complexity, which is justified by the accuracy improvement.

---

## Production Awareness & Real-World Constraints

This project is intentionally scoped as a real-time detection and systems-design exercise. In production trading infrastructure, several additional constraints would materially affect implementation decisions.

### 1. Fees & Slippage Modeling

The current implementation excludes trading fees and market impact. In practice, exchange fees vary by account tier and volume, maker vs taker fees significantly alter profitability, slippage must be modeled based on depth consumption, and partial fills introduce execution risk. A production-grade system would incorporate fee-aware profitability thresholds, dynamic slippage estimation, and conservative execution buffers. The exclusion here is intentional to keep focus on async architecture and depth-aware liquidity modeling.

### 2. Latency Sensitivity

The detector runs on a 1-second interval. In real markets, triangular arbitrage windows often close in milliseconds, and competing systems typically operate colocated with exchanges where network round-trip time is a dominant factor. A production implementation would require event-driven detection triggered on each book update, microsecond-level latency optimisation, and likely colocated infrastructure. For this project, a timer-based interval was chosen deliberately. Triggering detection on every book update was evaluated but rejected: at ~277 updates/second, event-driven scanning produced a significant CPU increase with no practical benefit for a demonstration tool. The 1-second interval keeps CPU usage negligible while still surfacing opportunities in a useful timeframe. The current design favours clarity and resource efficiency over ultra-low latency.

### 3. Hardcoded Triangular Paths

Arbitrage paths are currently defined statically:

```rust
let paths = vec![
    TriangularPath { base_pair: "BTC/USD", intermediate_pair: "ETH/BTC", quote_pair: "ETH/USD" },
    ...
];
```

A more general approach would dynamically derive all valid cycles from an adjacency graph built from the subscribed symbol list. Static paths were chosen here to keep detection predictable, scoped, and easy to reason about for demonstration purposes.

### 4. Execution Infrastructure

This system performs detection only. Production arbitrage requires atomic multi-leg execution, balance tracking per asset, capital allocation logic, risk controls (maximum exposure, kill switches), and retry and rollback strategies. Execution introduces significantly more complexity than detection and would fundamentally change the architecture.

### 5. Capital & Inventory Constraints

The demo assumes unconstrained inventory and ignores balance fragmentation. Real systems must consider available balances per asset, pre-positioned inventory to avoid transfer latency, opportunity cost of locked capital, and dynamic allocation across competing paths. A production system would integrate portfolio state directly into path evaluation.

---

## UI Design

### Why Ratatui?

Simple `println!` output scrolls continuously and is difficult to read for live data. A TUI provides a stable, structured view that updates in place.

`ratatui` was chosen for the following reasons:

- It is the de facto standard for Rust terminal UIs
- Lightweight with minimal dependencies
- Well-documented with active maintenance
- It has a funny name

### Refresh Rate

```rust
ui_refresh_interval_ms: 250  // 4 FPS
```

250ms strikes a reasonable balance: fast enough to feel responsive to live data, slow enough to avoid unnecessary CPU usage. Faster refresh rates would not improve the user experience, as the underlying detection interval is 1 second.

One potential enhancement would be dynamic terminal resize handling, though this is not essential for a demonstration.

---

## Testing Strategy

### Unit Tests

Unit tests cover the core logic in isolation:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_orderbook_sorting() { /* ... */ }
}
```

**Covered:**
- Order book sorting (bids descending, asks ascending)
- Checksum validation against the Kraken reference example
- Arbitrage detection accuracy (profitable paths found, unprofitable paths correctly rejected)
- Edge cases (empty books, zero quantity removal)

**Not covered:**
- WebSocket I/O (would require mocking the connection; checksum validation on the order book side provides indirect confidence in data correctness)
- UI rendering (impractical to assert terminal output)

The focus is on testing deterministic, pure logic where tests provide the highest signal.

### Integration Testing

A more complete test suite could include a mock WebSocket server that replays recorded Kraken messages, with assertions on the resulting order book state and detected opportunities. This has not been implemented, as the marginal value over the existing unit tests is limited for a demonstration project.
