# Polymarket Orderbook Viewer

A real-time Polymarket orderbook viewer built in Rust with a terminal user interface.

## Setup

1. **Clone the repository** (if not already done)

2. **Install Rust** (if not already installed)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **Set up environment variables**
   ```bash
   cp .env.example .env
   ```
   
   Then edit `.env` and add your private key:
   ```env
   PK=your_private_key_here_without_0x_prefix
   ```
   
   **Important:** Never commit your `.env` file or share your private key!

4. **Build the project**
   ```bash
   cargo build --release
   ```

## Usage

Run the orderbook viewer:
```bash
cargo run
```

Or with specific options:
```bash
cargo run -- --help
cargo run -- --token-id "your_token_id_here"
cargo run -- --interval 0.5 --depth 20
```

## Controls

### Market Selector
- `↑↓`: Navigate markets
- `PgUp/PgDn`: Fast scroll through markets
- `Enter`: Select market
- `/`: Search markets
- `Esc`: Exit search mode
- `q`: Quit

### Token Selector
- `↑↓`: Navigate tokens
- `PgUp/PgDn`: Fast scroll through tokens
- `Enter`: Select token
- `Backspace`: Back to market selector
- `q`: Quit

### Orderbook View
- `m`: Return to market selector
- `r`: Refresh orderbook
- `q`: Quit

## Features

- Real-time orderbook data from Polymarket
- Interactive market and token selection
- Search functionality for markets
- Visual depth chart
- Market statistics display
- Fast terminal-based UI

## Environment Variables

- `PK`: Your Ethereum private key (required)
- You can also specify a different environment variable name using `--private-key-env`

## Command Line Options

- `--token-id, -t`: Specific token ID to monitor
- `--interval, -i`: Update interval in seconds (default: 0.1)
- `--depth, -d`: Number of orders to show per side (default: 10)
- `--private-key-env`: Environment variable name for private key (default: "PK")
