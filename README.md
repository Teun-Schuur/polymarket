# Polymarket Real-time Orderbook Viewer

A high-performance, real-time terminal user interface (TUI) for monitoring Polymarket prediction markets, built in Rust. Features live orderbook data, price charts, Bitcoin integration, and an intuitive interface for market analysis.

## 🌟 Features

### Core Functionality
- **Real-time Orderbook Data**: Live streaming from Polymarket's CLOB API with WebSocket support
- **Market Discovery**: Browse and search through all active Polymarket prediction markets
- **Multi-token Support**: View orderbooks for both outcomes of binary prediction markets
- **Price History**: Track market price movements over time with interactive charts
- **Bitcoin Integration**: Live Bitcoin price feeds for crypto-related markets

### User Interface
- **Fast Terminal UI**: Built with Ratatui for responsive, terminal-based interaction
- **Tabbed Interface**: Switch between orderbook view and price history
- **Interactive Charts**: Visual orderbook depth charts and price history graphs
- **Search & Filter**: Quickly find markets with real-time search functionality
- **Responsive Design**: Adapts to different terminal sizes

### Technical Features
- **Modular Architecture**: Clean separation of concerns with organized module structure
- **WebSocket Streaming**: Real-time data updates with fallback to REST API
- **Error Handling**: Robust error recovery and user-friendly error messages
- **Performance Optimized**: Efficient data structures and minimal memory footprint
- **Cross-platform**: Works on Linux, macOS, and Windows

## 📋 Prerequisites

- **Rust** (1.70 or later): Install from [rustup.rs](https://rustup.rs/)
- **Terminal**: Any modern terminal emulator
- **Network**: Internet connection for API access

## 🚀 Quick Start

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd polymarket
   ```

2. **Set up environment variables**
   ```bash
   cp .env.example .env
   ```
   
   Edit `.env` and add your Ethereum private key:
   ```env
   PK=your_private_key_here_without_0x_prefix
   ```
   
   ⚠️ **Security Note**: Never commit your `.env` file or share your private key!

3. **Build and run**
   ```bash
   cargo run --release
   ```

## 🎮 Controls & Navigation

### Market Selector
| Key | Action |
|-----|--------|
| `↑↓` | Navigate through markets |
| `PgUp/PgDn` | Fast scroll (page up/down) |
| `Enter` | Select market |
| `/` | Open search mode |
| `Esc` | Exit search mode |
| `q` | Quit application |

### Token Selector
| Key | Action |
|-----|--------|
| `↑↓` | Navigate between token outcomes |
| `Enter` | Select token to view orderbook |
| `Backspace` | Return to market selector |
| `q` | Quit application |

### Orderbook View
| Key | Action |
|-----|--------|
| `◄►` or `h/l` | Switch between tabs (Orderbook/Price History) |
| `m` | Return to market selector |
| `r` | Refresh data |
| `q` | Quit application |

## 🏗️ Project Structure

```
src/
├── main.rs              # Application entry point
├── lib.rs               # Library exports
├── app.rs               # Core application logic and state management
├── cli.rs               # Command-line interface definitions
├── data.rs              # Data structures and models
├── ui/                  # User interface modules
│   ├── mod.rs           # UI module exports
│   ├── layout.rs        # Main layout and rendering logic
│   ├── selectors.rs     # Market and token selection interfaces
│   ├── orderbook.rs     # Orderbook display components
│   ├── charts.rs        # Chart rendering (price history, depth, Bitcoin)
│   └── components.rs    # Reusable UI components
└── websocket/           # WebSocket communication
    ├── mod.rs           # WebSocket module exports
    ├── clob.rs          # Polymarket CLOB WebSocket client
    └── bitcoin.rs       # Bitcoin price WebSocket client

external/
└── polymarket-rs-client/ # Polymarket API client library
```

## ⚙️ Configuration

### Environment Variables
- `PK`: Your Ethereum private key (required for authenticated API calls)

### Command Line Options
```bash
cargo run -- [OPTIONS]

Options:
  -t, --token-id <TOKEN_ID>      Specific token ID to monitor directly
  -i, --interval <SECONDS>       Update interval in seconds [default: 0.1]
  -d, --depth <NUMBER>           Number of orders to show per side [default: 10]
      --private-key-env <VAR>    Environment variable name for private key [default: "PK"]
  -h, --help                     Print help information
  -V, --version                  Print version information
```

### Examples
```bash
# Start with default settings
cargo run

# Monitor a specific token
cargo run -- --token-id "28159086305716095520316688285780453361496934489894720579037520569842658771360"

# Adjust update frequency and depth
cargo run -- --interval 0.5 --depth 20

# Use custom environment variable for private key
cargo run -- --private-key-env MY_PRIVATE_KEY
```

## 🔧 Development

### Building from Source
```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Check code formatting
cargo fmt --check

# Run clippy lints
cargo clippy
```

### Module Dependencies
- **ratatui**: Terminal user interface framework
- **tokio**: Async runtime for WebSocket and HTTP clients
- **clap**: Command-line argument parsing
- **serde**: JSON serialization/deserialization
- **chrono**: Date and time handling
- **crossterm**: Cross-platform terminal manipulation

## 🐛 Troubleshooting

### Common Issues

**"No markets found"**
- Check your internet connection
- Verify the Polymarket API is accessible
- Try running with `-i 1` for slower updates

**WebSocket connection errors**
- Application will fall back to REST API automatically
- Check firewall settings if WebSocket connections are blocked

**Private key errors**
- Ensure your `.env` file exists and contains a valid private key
- Verify the private key format (without `0x` prefix)
- Check file permissions on `.env`

**Build errors**
- Update Rust to the latest stable version: `rustup update`
- Clear build cache: `cargo clean && cargo build`

## 📊 Market Data

The application displays:
- **Bid/Ask Orders**: Live order book with prices and sizes
- **Market Statistics**: Spread, tick size, volume, last update time
- **Price Charts**: Historical price movements with timestamps
- **Bitcoin Integration**: Live BTC price for crypto-related markets
- **Market Status**: Active vs. closed market indicators

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes with proper tests
4. Follow Rust formatting: `cargo fmt`
5. Run clippy: `cargo clippy`
6. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🔗 Related Resources

- [Polymarket](https://polymarket.com/) - Prediction market platform
- [Polymarket API Documentation](https://docs.polymarket.com/) - Official API docs
- [Ratatui](https://ratatui.rs/) - Terminal UI framework
- [Rust Documentation](https://doc.rust-lang.org/) - Rust programming language

---

**Note**: This tool is for educational and analysis purposes. Always do your own research before making any trading decisions.
