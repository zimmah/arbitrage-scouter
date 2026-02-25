# Quick Start Guide

Get the Kraken Arbitrage Scouter running in under 5 minutes.

## Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Internet connection**: Required for the WebSocket connection to Kraken

> **Note**: No OpenSSL required! This project uses `rustls` (pure-Rust TLS).

## Installation

```bash
# Clone the repository
git clone https://github.com/zimmah/arbitrage-scouter.git
cd arbitrage-scouter

# Build in release mode (recommended)
cargo build --release

# Run the application
cargo run --release
```

## What You'll See

The terminal UI displays four panels:

1. **Header**: Uptime and keyboard controls
2. **Order Books**: Live bid/ask prices from Kraken
3. **Arbitrage Opportunities**: Detected profitable paths, if any exist
4. **Statistics**: Update count, opportunities found, best profit seen, and checksum integrity.

<img width="583" height="646" alt="screenshot of arbitrage sniper TUI" src="https://github.com/user-attachments/assets/42232271-d2aa-456c-a767-2b6d5f0d6ee3" />

## Controls

| Key | Action |
|---|---|
| `q` / `Q` / `Esc` | Quit gracefully |
| `Ctrl+C` | Force quit |

## Understanding the Output

### Order Books

```
BTC/USD       Bid:    47234.5000  Ask:    47241.3000  Spread:   14 bps
```

- **Bid**: Highest price a buyer is willing to pay
- **Ask**: Lowest price a seller is willing to accept
- **Spread**: Difference between ask and bid, expressed in basis points (1 bps = 0.01%)

### Arbitrage Opportunities

```
#1 Profit: 0.15%  Max: $1425.50
   BUY  BTC/USD     @ 47241.30000000
   BUY  ETH/BTC     @ 0.05203000
   SELL ETH/USD     @ 2461.50000000
```

- **Profit**: Expected percentage gain across the full trade path
- **Max**: Maximum tradeable amount, calculated from live order book depth
- **Path**: The sequence of trades required to realise the opportunity

### Why No Opportunities May Appear

Real arbitrage is rare. This is expected behaviour, not a bug. Common reasons include:

- HFT systems exploit openings near-instantly
- Transaction fees absorb marginal opportunities (fees are not modelled in this demo)
- Extended low-volatility periods reduce price divergence

## Configuration

To adjust detection parameters, edit the `Config` struct in `main.rs`:

```rust
let config = Config {
    min_profit_bps: 10,          // Minimum profit threshold to report (0.10%)
    detection_interval_ms: 1000, // How often to run detection (milliseconds)
    ui_refresh_interval_ms: 250, // UI refresh rate (250ms = 4 FPS)
};
```

Lowering `min_profit_bps` will surface more marginal opportunities for inspection.

## Troubleshooting

**The terminal UI is not rendering correctly.**  
Ensure your terminal is at least 80×24 characters. If the issue persists, try a different terminal emulator (iTerm2, Windows Terminal, Alacritty, etc.).

**Connection errors on startup.**  
Check your internet connection. The application will automatically attempt to reconnect every 5 seconds with exponential backoff.

**No opportunities are appearing.**  
This is normal — see [Why No Opportunities May Appear](#why-no-opportunities-may-appear) above. You can also lower `min_profit_bps` to see more marginal cases.

**Build errors.**  
```bash
# Update Rust to the latest stable version
rustup update

# Clean the build cache and rebuild
cargo clean
cargo build --release
```

## Next Steps

- [README.md](README.md) — Full project documentation
- [DESIGN.md](DESIGN.md) — Architecture and technical decisions
- `src/` — Source code
- `cargo test` — Run the test suite

---

> ⚠️ **Educational Use Only**: This project is for learning and demonstration purposes. Do not use it for actual trading.
