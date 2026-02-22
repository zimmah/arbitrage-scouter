# Quick Start Guide

Get the Kraken Arbitrage Scout running in under 5 minutes.

## Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **Internet connection**: Needed for WebSocket connection to Kraken

**Note**: No OpenSSL required! This project uses `rustls` (pure Rust TLS).

## Installation

```bash
# Clone the repository
git clone https://github.com/zimmah/arbitrage-scout.git
cd arbitrage-scout

# Build in release mode (optimized)
cargo build --release

# Run the application
cargo run --release
```

## What You'll See

The terminal UI will display:

1. **Header**: Uptime and controls
2. **Order Books**: Live bid/ask prices from Kraken
3. **Arbitrage Opportunities**: Detected profitable paths (if any)
4. **Statistics**: Updates, opportunities found, best profit

## Controls

- **`q`** or **`Q`**: Quit
- **`Esc`**: Quit  
- **`Ctrl+C`**: Force quit

## Understanding the Output

### Order Books

```
BTC/USD       Bid:    47234.5000  Ask:    47241.3000  Spread:   14 bps
```

- **Bid**: Highest price someone will pay
- **Ask**: Lowest price someone will sell
- **Spread**: Difference (in basis points, 1 bps = 0.01%)

### Arbitrage Opportunities

```
#1 Profit: 0.15%  Max: $1425.50
   BUY  BTC/USD     @ 47241.30000000
   BUY  ETH/BTC     @ 0.05203000
   SELL ETH/USD     @ 2461.50000000
```

- **Profit**: Percentage gain
- **Max**: Maximum amount tradeable (based on order book depth)
- **Path**: Sequence of trades to exploit the opportunity

### Why You Might Not See Opportunities

Real arbitrage is rare because:
1. **HFT bots** exploit them instantly
2. **Transaction fees** eat into profits (not shown in this demo)
3. **Low volatility** periods

This is normal and expected!

## Configuration

Edit `main.rs` to adjust:

```rust
let config = Config {
    min_profit_bps: 10,          // Minimum profit to report (0.10%)
    detection_interval_ms: 1000, // How often to check (1 second)
    ui_refresh_interval_ms: 250, // UI refresh rate (250ms = 4 FPS)
};
```

## Troubleshooting

### Terminal looks weird
- Make sure your terminal is at least 80x24 characters
- Try a different terminal (iTerm2, Windows Terminal, etc.)

### Connection errors
- Check your internet connection
- Kraken might be down (rare)
- The app will auto-reconnect every 5 seconds

### No opportunities showing
- This is normal! See "[Why You Might Not See Opportunities](#why-you-might-not-see-opportunities)" above
- Try lowering `min_profit_bps` to see more marginal opportunities

### Build errors
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

## Next Steps

- Read [README.md](README.md) for full documentation
- Read [DESIGN.md](DESIGN.md) to understand the architecture
- Explore the code in `src/`
- Run tests: `cargo test`

## Questions?

Open an issue on GitHub or check the documentation in the README.

---

**Remember**: This is for educational purposes only. Do not use for actual trading!
