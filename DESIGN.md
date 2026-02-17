# Design Document

This document explains the technical decisions, tradeoffs, and async patterns used in this project.

## Table of Contents
1. [Architecture Overview](#architecture-overview)
2. [Data Flow](#data-flow)
3. [Performance Considerations](#performance-considerations)
4. [Depth Calculation Design](#depth-calculation-design)
5. [UI Design](#ui-design)
6. [Testing Strategy](#testing-strategy)


## Architecture Overview

### Module Responsibilities

```
main.rs          - Orchestrates tasks, owns shared state
types.rs         - Data structures, no logic
orderbook.rs     - Order book state management
arbitrage.rs     - Detection algorithms
websocket.rs     - Network I/O and protocol handling
ui.rs            - Terminal rendering and input
```

**Principle**: Each module has one clear responsibility. No circular dependencies.

### Why This Structure?

- **Testability**: Pure functions in `arbitrage.rs` can be tested without I/O
- **Clarity**: New contributors can understand each module independently
- **Maintainability**: Changes to UI don't affect arbitrage logic
- **Reusability**: `arbitrage.rs` could be reused in a different UI

## Data Flow

### WebSocket → OrderBook → Arbitrage → UI

```
1. Kraken WebSocket sends order book update
   ↓
2. websocket.rs parses JSON
   ↓
3. OrderBookManager.update_book() (write lock)
   ↓
4. Detector reads books every 1s (read lock)
   ↓
5. Detector calculates opportunities
   ↓
6. Detector updates own state (write lock)
   ↓
7. UI reads both books and opportunities (read locks)
   ↓
8. UI renders to terminal
```

**Decoupling**: UI never talks to WebSocket directly. Each component only knows about immediate dependencies.

## Performance Considerations

### Memory Usage

**Order Books**: 
- 8 pairs × 2 sides × 10 levels × ~32 bytes ≈ 5 KB
- Negligible

**Opportunities**:
- Stored in Vec, typically < 10 items
- Each opportunity ~200 bytes
- Negligible

**Total**: < 10 MB including Tokio runtime

### CPU Usage

**Bottleneck**: Arbitrage detection (runs every 1s)
- Checks ~8 triangular paths
- Each path: ~10 floating point operations
- Total: ~80 FLOPs per second
- Result: < 1% CPU on modern hardware

**Optimization Opportunity**: Could parallelize path checking with `rayon`, but unnecessary here.

### Network

**Bandwidth**: ~1-5 KB/s from WebSocket (compressed JSON)

**Latency**: Not critical (we're not executing trades)

### Lock Contention

**Read Locks**: Never contend (multiple readers allowed)

**Write Locks**: 
- WebSocket: ~10-100 updates/second
- Detector: 1 update/second
- Total write lock time: < 0.1% of time

**Result**: Lock contention is not a bottleneck

## Depth Calculation Design

### The Problem

Original code used a magic constant:

```rust
let initial_amount = 1000.0; // BAD: Arbitrary magic number
```

**Issues**:
1. If depth is $100, profit calculation is wrong
2. If depth is $10,000, we underestimate opportunity
3. Doesn't reflect real trading constraints

### The Solution

Calculate max executable based on order book depth:

```rust
// Work backwards from the final step
let max_eth_sell = bid3.quantity;
let max_btc_for_step2 = max_eth_sell * ask2.price;
let max_executable_usd = /* minimum of all constraints */;
```

**Why This Works**:
1. Respects liquidity at each step
2. Finds the bottleneck
3. Realistic maximum profit calculation

**Tradeoff**: More complex code, but significantly more accurate.

## UI Design

### Why Ratatui?

Using `println!` is simple, but the output is messy and hard to read, as the text will keep moving constantly, a TUI is much more pleasant to read.


**Choice**: `ratatui`
- Industry standard for Rust TUIs
- Lightweight
- Good documentation
- Active maintenance

Alternatives like `cursive` exist, but `ratatui` is just a great name and it works, so why not.

### Refresh Rate

```rust
ui_refresh_interval_ms: 250  // 4 FPS
```

**Why 250ms?**:
- Fast enough to feel responsive
- Slow enough to avoid excessive CPU
- Faster updates would not improve UX, users wouldn't be able to parse the information that fast anyway

**Tradeoff**: Could detect terminal size changes dynamically, but not essential for demo.

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_orderbook_sorting() { /* ... */ }
}
```

**What's Tested**:
- Order book sorting logic
- Profitable path detection
- Edge cases (empty books, zero quantity)

**What's Not Tested**:
- WebSocket I/O (would need mocking)
- UI rendering (hard to test)

**Tradeoff**: 100% test coverage is expensive. We test core logic only.

### Integration Testing

Could add:
```bash
# Start mock WebSocket server
# Run application
# Verify it connects and processes data
```

**Not Implemented**: Diminishing returns for a demo project.