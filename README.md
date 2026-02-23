# Kraken Arbitrage Scouter

A real-time cryptocurrency arbitrage detection system built in Rust, showcasing professional async programming with Tokio.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)

## Overview

This project demonstrates production-quality async Rust development through a live arbitrage detection system. It connects to Kraken's WebSocket API, monitors order books, and detects triangular arbitrage opportunities in real-time.

**⚠️ Educational Purpose Only**: This is a portfolio project, not a trading system. It does not execute trades and is not financial advice.

## Key Features

### Async Rust Patterns Demonstrated

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
- No arbitrary "magic number" constant
- Properly accounts for liquidity constraints
- Demonstrates depth-aware calculations   

**Tradeoff**: More complex logic, but significantly more accurate

### 3. Locking Strategy
**Decision**: Use `RwLock` instead of `Mutex`  
**Reasoning**: 
- Read-heavy workload (UI and detector read, only WebSocket writes)
- Multiple concurrent readers don't block each other
- Better performance for this access pattern   

**Tradeoff**: Write operations slightly slower, but we have few writes

### 4. TUI vs. Logging
**Decision**: Use `ratatui` for terminal UI instead of `println!` logging  
**Reasoning**: 
- Professional, stable display
- Better user experience for live data
- Showcases additional Rust ecosystem knowledge   

**Tradeoff**: More complex than simple logging, but worth it for demo quality

### 5. Error Handling Strategy
**Decision**: Use `anyhow::Result` for application-level errors  
**Reasoning**: 
- Simple, ergonomic error handling
- Good for applications
- Easy context addition with `.context()`   

**Tradeoff**: Less type-safe than custom error enums, but appropriate for apps

### 6. No Authentication
**Decision**: Only use public WebSocket endpoints  
**Reasoning**: 
- No credentials needed for order book data
- Simpler setup for anyone running the code
- Sufficient for arbitrage detection   

**Tradeoff**: Can't show private endpoint patterns, but keeps demo accessible

### 7. Limited Order Book Depth
**Decision**: Keep only top 10 price levels per side  
**Reasoning**: 
- Sufficient for realistic arbitrage calculation
- Reduces memory usage
- Top-of-book is most relevant for arbitrage   

**Tradeoff**: Might miss deep book opportunities, but they're rare and hard to execute

### 8. Ignore fees
**Decision**: Ignore trading fees in the calculations   
**Reasoning**:
- Fees are beyond the scope of this exercise
- This demo is not intended to be used in real life
- Fees depend on specific conditions anyway

**Tradeoff**: Easier to implement, but less accurate detection

## Project Structure

```
src/
├── main.rs         - Application entry point and task orchestration
├── types.rs        - Core data structures and configuration
├── orderbook.rs    - Order book management and guaranteeing data accuracy
├── arbitrage.rs    - Triangular arbitrage detection logic
├── websocket.rs    - WebSocket client and reconnection logic
└── ui.rs           - Terminal UI rendering with ratatui
```

Each module has a single, clear responsibility. Dependencies flow downward (no circular deps).

## Building & Running

### Prerequisites
- Rust 1.75 or later
- Internet connection (for WebSocket)

**Note**: This project uses `rustls` for TLS, so you don't need OpenSSL installed.

### Commands

```bash
# Clone the repository
git clone https://github.com/zimmah/arbitrage-scouter
cd arbitrage-scouter

# Build in release mode (optimized)
cargo build --release

# Run the application
cargo run --release

# Run tests
cargo test

# Press 'q' or Ctrl+C to exit gracefully
```

A width of at least 80 characters is recommended

## Sample Output

```
┌────────────────────────────────────────────────────────────────────────┐
│ Kraken Arbitrage Scouter | Press 'q' to quit | Uptime: 2m 34s          │
└────────────────────────────────────────────────────────────────────────┘
┌─ Order Books (Live) ───────────────────────────────────────────────────┐
│ BTC/USD       Bid:    47234.5000  Ask:    67241.3000  Spread:  1.4 bps │
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
| Valid Checksums: ✅ All valid                                          |
└────────────────────────────────────────────────────────────────────────┘
```
<img width="657" height="710" alt="image" src="https://github.com/user-attachments/assets/47906179-f944-436a-badc-de7de281dbb5" />


## Arbitrage Detection Logic

### Triangular Arbitrage Explained

The system detects profit opportunities in currency triangles:

**Example Path**: USD → BTC → ETH → USD

**Forward Direction**:
1. Buy BTC with USD (at ask price of BTC/USD)
2. Buy ETH with BTC (at ask price of ETH/BTC)
3. Sell ETH for USD (at bid price of ETH/USD)
4. Profit = final USD - initial USD

**Reverse Direction**:
1. Buy ETH with USD (at ask price of ETH/USD)
2. Sell ETH for BTC (at bid price of ETH/BTC)
3. Sell BTC for USD (at bid price of BTC/USD)
4. Profit = final USD - initial USD

### Depth-Aware Calculation

Unlike naive implementations that use a fixed amount (such as $1000), this system:
1. Works backward from the final step
2. Finds the bottleneck (minimum liquidity) across all steps
3. Calculates max executable amount based on real order book depth
4. Reports both profit percentage AND maximum tradeable volume

This approach is more realistic and demonstrates understanding of market microstructure.

## Configuration

Modify the `Config` struct in `main.rs`:

```rust
let config = Config {
    min_profit_bps: 10,              // 0.10% minimum (10 basis points)
    detection_interval_ms: 1000,     // Check every 1 second
    ui_refresh_interval_ms: 250,     // 4 FPS refresh rate
};
```

## Why No Credentials?

Kraken's WebSocket API has two types of endpoints:

1. **Public endpoints** (no auth): Market data, order books, trades
2. **Private endpoints** (auth required): Account info, order placement

This project uses only public endpoints to:
- Make it runnable without API keys
- Keep the code simple and focused
- Avoid security concerns with credential storage or leakage

## Learning Resources

This project demonstrates concepts from:
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial) - Async runtime
- [Async Book](https://rust-lang.github.io/async-book/) - Async patterns
- [Ratatui Book](https://ratatui.rs/) - Terminal UIs
- [Kraken API Docs](https://docs.kraken.com/websockets-v2/) - WebSocket protocol

## Potential Enhancements

For a production system, consider adding:

- [ ] Multiple exchange support
- [ ] Cross-exchange arbitrage detection
- [ ] Slippage and fee modeling
- [ ] Historical data persistence (SQLite/PostgreSQL)
- [ ] Prometheus metrics export
- [ ] Alert notifications (email, Slack, Discord)
- [ ] Web dashboard with live WebSocket streaming
- [ ] Configuration file support (TOML/YAML)
- [ ] Trade execution simulation with backtesting
- [ ] Actual trade execution

## Testing

```bash
# Run unit tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Test a specific module
cargo test orderbook::tests
```

Tests cover:
- Order book sorting (bids descending, asks ascending)
- Checksum accuracy
- Profitable path detection

## Performance Considerations

- **Memory**: ~10 MB for 8 order books with 10 levels each
- **CPU**: Minimal (< 5% on modern CPUs)
- **Network**: ~1-5 KB/s for WebSocket data
- **Latency**: Detection runs every 1 second (configurable)

## Troubleshooting

**Q: Terminal UI not rendering correctly?**  
A: Make sure your terminal supports ANSI colors and has at least 80x24 size.

**Q: WebSocket connection failing?**  
A: Check your internet connection. The code will automatically retry every 5 seconds.

**Q: No opportunities detected?**  
A: This is normal! Real arbitrage is rare due to:
- Transaction fees (not modeled in this demo)
- Fast HFT bots that exploit opportunities instantly
- Low volatility periods

**Q: Can I use this for actual trading?**  
A: **No.** This is educational only. Real trading requires:
- Fee calculations
- Slippage modeling
- Risk management
- Proper execution infrastructure
- Significant capital

## License

MIT License - see LICENSE file for details

## Disclaimer

This software is for educational purposes only. It is not financial advice. Cryptocurrency trading carries significant risk. Do not use this for actual trading without proper understanding of the markets, risk management, and sufficient capital.

## Contributing

This is a portfolio project, but feedback and suggestions are welcome! Open an issue or submit a pull request.

## Author

Zimmah

Built with ❤️ using Rust and Tokio
