# Design Document

This document covers the technical decisions, architectural tradeoffs, and async patterns used in this project.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Data Flow](#data-flow)
3. [Performance Considerations](#performance-considerations)
4. [Depth Calculation Design](#depth-calculation-design)
5. [UI Design](#ui-design)
6. [Testing Strategy](#testing-strategy)

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

- **Testability**: Pure functions in `arbitrage.rs` and `orderbook.rs` can be unit tested without any I/O
- **Clarity**: Each module can be understood in isolation
- **Maintainability**: UI changes do not affect arbitrage logic, and vice versa
- **Reusability**: `arbitrage.rs` and `orderbook.rs` could be reused with an entirely different frontend

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

The arbitrage detector is the primary computational component. It runs every second, checks approximately 8 triangular paths, and performs roughly 10 floating-point operations per path — around 80 FLOPs per second in total. In practice this consumes less than 1% CPU on modern hardware.

If parallelism were needed, path checking could be distributed across threads using `rayon`. This is unnecessary at the current scale.

### Network

WebSocket traffic from Kraken is approximately 1–5 KB/s of compressed JSON. Latency is not a concern since this system does not execute trades.

### Lock Contention

`RwLock` is used throughout to optimise for the read-heavy access pattern:

- **Read locks** never contend with each other (multiple concurrent readers are permitted)
- **Write locks** are acquired infrequently: the WebSocket client writes on each update (typically ~277 times/second), and the detector writes once per second

Total write lock hold time is well under 0.1% — lock contention is not a bottleneck.

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
- WebSocket I/O (would require mocking the connection; checksum validation
  on the order book side provides indirect confidence in data correctness)
- UI rendering (impractical to assert terminal output)

Full test coverage would be disproportionately expensive relative to the value for a project of this scope. The focus is on testing deterministic, pure logic.

### Integration Testing

A more complete test suite could include a mock WebSocket server that replays recorded Kraken messages, with assertions on the resulting order book state and detected opportunities. This has not been implemented, as the marginal value over the existing unit tests is limited.