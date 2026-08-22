# Kraken Arbitrage Scouter

A real-time cryptocurrency arbitrage detection system built in Rust using async Tokio. Connects to Kraken's WebSocket API, maintains live order books, and identifies triangular arbitrage opportunities as they emerge.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://img.shields.io/badge/license-MIT-blue.svg)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://img.shields.io/badge/rust-1.75+-orange.svg)

> ⚠️ **Educational Use Only**: This tool is designed for learning and demonstration purposes. It does not execute trades and should not be construed as financial advice.

## Overview

This project demonstrates production-quality async Rust development through a live arbitrage detection system. It connects to Kraken's WebSocket API, monitors order books, and detects triangular arbitrage opportunities in real-time.

## Key Features

### Async Rust Patterns

1. **Multi-Task Concurrency**
   - Independent async tasks for WebSocket handling, arbitrage detection, and TUI rendering
   - Task coordination using `tokio::spawn` and `tokio::select!`
   - Graceful shutdown propagation across all tasks

2. **Shared State Management**
   - Thread-safe state with `Arc<RwLock<T>>`
   - Read-optimized locking patterns for performance
   - Zero-copy access where possible

3. **WebSocket Client**
   - Persistent connection with automatic reconnection
   - Non-blocking message processing
   - Proper ping/pong handling

4. **Terminal UI (TUI)**
   - Flicker-free rendering with `ratatui`
   - Async-compatible event loop
   - Clean separation of data and presentation

5. **Error Handling**
   - Comprehensive error propagation with `anyhow`
   - Graceful degradation on transient failures
   - Automatic reconnection with backoff

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Main Application                     │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  WebSocket   │  │  Arbitrage   │  │   Terminal   │   │
│  │    Task      │  │   Detector   │  │   UI Task    │   │
│  │  (receive)   │  │   (compute)  │  │  (render)    │   │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │
│         │                 │                 │           │
│         └─────────────────┴─────────────────┘           │
│                           │                             │
│                ┌──────────▼──────────┐                  │
│                │  OrderBookManager   │                  │
│                │   (Arc<RwLock>)     │                  │
│                └─────────────────────┘                  │
└─────────────────────────────────────────────────────────┘
```

## Design Decisions & Tradeoffs

### 1. WebSocket Library Choice

**Decision**: Use `tokio-tungstenite` instead of `fast_websocket_client`  
**Reasoning**:
- Native async/await support (no manual frame handling)
- Better maintained and more widely used
- Cleaner API for splitting read/write streams

**Tradeoff**: Slightly higher-level abstraction, less control over frames

### 2. Depth Calculation

**Decision**: Calculate max executable amount based on actual order book depth  
**Reasoning**:
- Avoids arbitrary constant amounts (a.k.a "magic numbers")
- Properly accounts for liquidity constraints
- Demonstrates depth-aware calculation

**Tradeoff**: More complex logic, but significantly more accurate

### 3. Locking Strategy

**Decision**: Use `RwLock` instead of `Mutex`  
**Reasoning**:
- Read-heavy workload (UI and detector read; only WebSocket writes)
- Multiple concurrent readers do not block each other
- Better performance for this access pattern

**Tradeoff**: Write operations are slightly slower, but writes are infrequent

### 4. TUI vs. Logging

**Decision**: Use `ratatui` for terminal UI instead of `println!` logging  
**Reasoning**:
- Stable, flicker-free display for live data
- Better user experience overall
- Demonstrates familiarity with the Rust TUI ecosystem

**Tradeoff**: More complex than simple logging, but the result quality justifies it

### 5. Error Handling Strategy

**Decision**: Use `anyhow::Result` for application-level errors  
**Reasoning**:
- Ergonomic error handling well-suited to application code
- Easy context addition with `.context()`

**Tradeoff**: Less type-safe than custom error enums, but appropriate for this use case

### 6. Public Endpoints Only

**Decision**: Use only public WebSocket endpoints (no authentication)  
**Reasoning**:
- No credentials required to run the project
- Keeps the codebase simple and accessible
- Order book data from public endpoints is sufficient for arbitrage detection

**Tradeoff**: Private endpoint patterns are not demonstrated, but this keeps the demo easy to run

### 7. Limited Order Book Depth

**Decision**: Keep only the top 10 price levels per side  
**Reasoning**:
- Sufficient for realistic arbitrage calculation
- Reduces memory usage
- Top-of-book data is most relevant for arbitrage detection

**Tradeoff**: Deep-book opportunities may be missed, though these are rare and difficult to execute in practice

### 8. Fee Exclusion

**Decision**: Exclude trading fees from calculations  
**Reasoning**:
- Fee modeling is out of scope for this demonstration
- Fees vary significantly based on account tier and market conditions

**Tradeoff**: Simpler implementation, at the cost of detection accuracy

## Project Structure

```
src/
├── main.rs         - Application entry point and task orchestration
├── types.rs        - Core data structures and configuration
├── orderbook.rs    - Order book management and data integrity
├── arbitrage.rs    - Triangular arbitrage detection logic
├── websocket.rs    - WebSocket client and reconnection logic
└── ui.rs           - Terminal UI rendering with ratatui
```

Each module has a single, clear responsibility. Dependencies flow in one direction with no circular references.

## Building & Running

### Prerequisites

- Rust 1.75 or later
- An active internet connection (for WebSocket connectivity)

> **Note**: This project uses `rustls` for TLS, so OpenSSL is not required.

### Commands

```bash
# Clone the repository
git clone https://github.com/zimmah/arbitrage-scouter
cd arbitrage-scouter

# Build in release mode (recommended)
cargo build --release

# Run the application
cargo run --release

# Run tests
cargo test

# Press 'q' or Ctrl+C to exit gracefully
```

A terminal width of at least 80 characters is recommended.

## Sample Output

```
┌────────────────────────────────────────────────────────────────────────┐
│ Kraken Arbitrage Scouter | Press 'q' to quit | Uptime: 2m 34s          │
└────────────────────────────────────────────────────────────────────────┘
┌─ Order Books (Live) ───────────────────────────────────────────────────┐
│ BTC/USD       Bid:    47234.5000  Ask:    47241.3000  Spread:  1.4 bps │
│ ETH/USD       Bid:     2456.7800  Ask:     2457.9200  Spread:  4.6 bps │
│ ETH/BTC       Bid:        0.0520  Ask:        0.0521  Spread:  1.9 bps │
└────────────────────────────────────────────────────────────────────────┘
┌─ Arbitrage Opportunities ──────────────────────────────────────────────┐
│ #1 Profit: 0.15%  Max: $1425.50                                        │
│    BUY  BTC/USD     @ 47241.30000000                                   │
│    BUY  ETH/BTC     @ 0.05203000                                       │
│    SELL ETH/USD     @ 2461.50000000                                    │
│                                                                        │
│ #2 Profit: 0.08%  Max: $892.30                                         │
│    BUY  ETH/USD     @ 2457.92000000                                    │
│    SELL ETH/BTC     @ 0.05199000                                       │
│    SELL BTC/USD     @ 47234.50000000                                   │
└────────────────────────────────────────────────────────────────────────┘
┌─ Statistics ───────────────────────────────────────────────────────────┐
│ Order Book Updates: 2847                                               │
│ Opportunities Found: 12                                                │
│ Best Opportunity: 0.23%                                                │
│ Valid Checksums: ✅ All valid                                          │
└────────────────────────────────────────────────────────────────────────┘
```
<img width="583" height="646" alt="screenshot of arbitrage sniper TUI" src="https://github.com/user-attachments/assets/42232271-d2aa-456c-a767-2b6d5f0d6ee3" />

## Arbitrage Detection Logic

### Triangular Arbitrage Explained

The system detects profit opportunities within currency triangles.

**Example Path**: USD → BTC → ETH → USD

**Forward Direction**:
1. Buy BTC with USD (at the ask price of BTC/USD)
2. Buy ETH with BTC (at the ask price of ETH/BTC)
3. Sell ETH for USD (at the bid price of ETH/USD)
4. Profit = final USD - initial USD

**Reverse Direction**:
1. Buy ETH with USD (at the ask price of ETH/USD)
2. Sell ETH for BTC (at the bid price of ETH/BTC)
3. Sell BTC for USD (at the bid price of BTC/USD)
4. Profit = final USD - initial USD

### Depth-Aware Calculation

Unlike naive implementations that use a fixed notional amount, this system:

1. Works backward from the final step
2. Identifies the bottleneck (minimum liquidity) across all steps
3. Calculates the maximum executable amount based on real order book depth
4. Reports both the profit percentage and the maximum tradeable volume

This approach is more realistic and reflects an understanding of market microstructure.

## Configuration

Modify the `Config` struct in `main.rs`:

```rust
let config = Config {
    min_profit_bps: 10,              // 0.10% minimum (10 basis points)
    detection_interval_ms: 1000,     // Check every 1 second
    ui_refresh_interval_ms: 250,     // 4 FPS refresh rate
};
```

## Testing

```bash
# Run all unit tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Test a specific module
cargo test orderbook::tests
```

Test coverage includes:

- Order book sorting (bids descending, asks ascending)
- Checksum accuracy
- Profitable path detection

## Performance

| Metric | Value |
|---|---|
| Memory | ~10 MB (8 order books × 10 levels) |
| CPU usage | < 5% on modern hardware |
| Network | ~1–5 KB/s WebSocket data |
| Detection latency | Configurable (default: 1 second) |

## Troubleshooting

**Q: The terminal UI is not rendering correctly.**  
A: Ensure your terminal supports ANSI colors and is at least 80×24 characters in size.

**Q: The WebSocket connection keeps failing.**  
A: Check your internet connection. The application will automatically retry every 5 seconds.

**Q: No arbitrage opportunities are being detected.**  
A: This is expected. Real arbitrage opportunities are rare due to transaction fees (not modeled in this demo), HFT bots that exploit openings near-instantly, and extended low-volatility periods.

**Q: Can I use this for actual trading?**  
A: No. This project is for educational purposes only. Production trading requires fee and slippage modeling, risk management, execution infrastructure, and sufficient capital.

## Potential Enhancements

For a production system, consider adding:

- Multi-exchange support with cross-exchange arbitrage detection
- Fee and slippage modeling
- Historical data persistence (SQLite/PostgreSQL)
- Prometheus metrics export
- Alert notifications (email, Slack, Discord)
- Web dashboard with live WebSocket streaming
- Configuration file support (TOML/YAML)
- Backtesting with historical order book data
- Live trade execution with configurable risk controls

## Learning Resources

Concepts demonstrated in this project draw from:

- [Tokio Tutorial](https://tokio.rs/tokio/tutorial) — Async runtime
- [Async Book](https://rust-lang.github.io/async-book/) — Async patterns
- [Ratatui Book](https://ratatui.rs/) — Terminal UIs
- [Kraken API Docs](https://docs.kraken.com/websockets-v2/) — WebSocket protocol

## Contributing

Feedback, suggestions, and contributions are welcome. Feel free to open an issue or submit a pull request.

## License

MIT — see [LICENSE](LICENSE) for details.

## Disclaimer

This software is for educational purposes only and does not constitute financial advice. Cryptocurrency trading carries significant risk. Do not use this for actual trading without a thorough understanding of the markets, proper risk management, and sufficient capital.

## Author

[Zimmah](https://github.com/zimmah) — Built with ❤️ using Rust and Tokio.
