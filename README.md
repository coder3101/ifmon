# ifmon - Interface Monitor

A beautiful terminal-based network interface monitoring tool built with Rust and [ratatui](https://ratatui.rs/).

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)

## Features

- 📊 **Real-time network speed monitoring** - Track RX/TX speeds with live updates
- 📈 **Beautiful line graphs** - Smooth Braille-dot graphs showing speed history
- 🎯 **Dynamic scaling** - Graphs automatically adapt to your connection speed
- 🔍 **Interface filtering** - Smart filtering shows only relevant physical interfaces
- 📡 **Multiple interfaces** - Switch between network interfaces with Tab
- 🎨 **Clean TUI** - Professional terminal user interface with color-coded stats
- ⚡ **Lightweight** - Minimal resource usage, perfect for monitoring

## Installation

### From crates.io

```bash
cargo install ifmon
```

### From source

```bash
git clone https://github.com/coder3101/ifmon.git
cd ifmon
cargo build --release
```

The binary will be available at `target/release/ifmon`.

## Usage

Simply run:

```bash
ifmon
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` | Switch to next interface |
| `Shift+Tab` | Switch to previous interface |
| `f` | Toggle show all/physical interfaces only |
| `h` | Show help screen |
| `↑` / `↓` | Scroll through IP addresses |
| `q` or `Esc` | Quit |

## Screenshots

The interface shows:
- **Interface Information** - Name, type, MAC address, MTU, state, IP addresses
- **RX Graph** - Download speed over time with current/peak/total stats
- **TX Graph** - Upload speed over time with current/peak/total stats

All statistics update in real-time every 500ms.

## Requirements

- Rust 1.70 or higher
- A terminal that supports Unicode and colors
- Works on Linux, macOS, and BSD systems

## Building

```bash
cargo build --release
```

For optimized binary size (already configured):

```bash
cargo build --release
```

## Development

The project is organized into clean modules:

```
src/
├── main.rs          - Entry point
├── app.rs           - Application state and main loop
├── types/           - Data structures (SpeedHistory)
├── network/         - Network interface filtering
├── ui/              - UI rendering (graphs, interface info, help)
└── utils/           - Formatting utilities
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Author

**Ashar Khan** - [coder3101](https://github.com/coder3101)

## Acknowledgments

- Built with [ratatui](https://ratatui.rs/) - An amazing Rust TUI framework
- Uses [netdev](https://crates.io/crates/netdev) for network interface statistics
- Inspired by classic network monitoring tools like `iftop` and `nethogs`
