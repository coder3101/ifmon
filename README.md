# ifmon - Interface Monitor

> A beautiful terminal-based network interface monitoring tool built with Rust and [ratatui](https://ratatui.rs/).

[![Crates.io](https://img.shields.io/crates/v/ifmon.svg)](https://crates.io/crates/ifmon)
[![Downloads](https://img.shields.io/crates/d/ifmon.svg)](https://crates.io/crates/ifmon)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)

![ifmon screenshot](assets/ifmon.png)

## Features

- 📊 **Real-time Monitoring** - Track RX/TX speeds with live updates every 500ms
- 📈 **Beautiful Graphs** - Smooth Braille-dot line charts showing speed history over time
- 🎯 **Dynamic Scaling** - Y-axis automatically adapts to your actual connection speed
- 🔍 **Smart Filtering** - Intelligently filters out loopback, tunnel, and virtual interfaces
- 📡 **Multi-Interface** - Easily switch between all your network interfaces
- 🌐 **IPv4 & IPv6** - Full support for both address families with scrollable display
- 🎨 **Clean Interface** - Professional TUI with color-coded stats and minimal design
- ⚡ **Lightweight** - Minimal CPU and memory footprint, perfect for always-on monitoring
- 🔧 **Cross-Platform** - Works on Linux, macOS, and BSD systems

## Quick Start

```bash
# Install from crates.io
cargo install ifmon

# Run it
ifmon
```

That's it! Use `Tab` to switch interfaces, `f` to filter, `h` for help.

## Installation

### From crates.io (Recommended)

```bash
cargo install ifmon
```

### From source

```bash
git clone https://github.com/coder3101/ifmon.git
cd ifmon
cargo build --release
# Binary will be at: target/release/ifmon
```

### Pre-built Binaries

Pre-built binaries for Linux (x86_64, ARM64), macOS (Intel, Apple Silicon), and Windows are available on the [releases page](https://github.com/coder3101/ifmon/releases).

## Usage

Simply run:

```bash
ifmon
```

The interface displays three main sections:
1. **Interface Info** - Shows name, type, MAC, MTU, state, and IP addresses
2. **RX Graph** - Download speeds with current/peak/total statistics  
3. **TX Graph** - Upload speeds with current/peak/total statistics

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` | Switch to next interface |
| `Shift+Tab` | Switch to previous interface |
| `f` | Toggle between all interfaces / physical only |
| `h` | Show/hide help screen |
| `↑` / `↓` | Scroll through IP addresses (when multiple IPs) |
| `q` or `Esc` | Quit application |

### What Gets Displayed

**Physical interfaces mode** (default):
- Ethernet, WiFi, and other physical network adapters
- Automatically filters out loopback (lo), tunnels, and virtual interfaces
- Includes any interface with assigned IP addresses

**All interfaces mode** (press `f`):
- Shows everything including loopback, bridges, tunnels, etc.

All statistics update in real-time every 500ms with dynamic graph scaling.

## Why ifmon?

Modern network monitoring tools are often complex or require root privileges. `ifmon` provides a simple, zero-configuration solution:

- **No sudo required** - Works with user-level permissions
- **Zero configuration** - Just run it, no setup needed
- **Clean output** - Focus on what matters: current speeds and trends
- **Terminal-native** - Perfect for remote SSH sessions or tmux setups
- **Efficient** - Uses Braille characters for high-resolution graphs in minimal space

## Technical Details

- **Update interval**: 500ms
- **History window**: 100 data points (50 seconds)
- **Graph rendering**: Braille dot markers (2×4 resolution per character)
- **Scaling**: Fully dynamic based on observed traffic
- **Platform support**: Cross-platform via `netdev` crate

## Requirements

- **Rust**: 1.70 or higher (for building)
- **Terminal**: Unicode and color support recommended
- **OS**: Linux, macOS, or BSD systems

## Building from Source

```bash
git clone https://github.com/coder3101/ifmon.git
cd ifmon
cargo build --release
```

The release binary includes size optimizations (LTO, stripped symbols) and will be available at `target/release/ifmon`.

## Project Structure

Clean, modular architecture for maintainability:

```
src/
├── main.rs              Entry point and error handling
├── app.rs               Application state and event loop
├── types/
│   └── history.rs       SpeedHistory data structure
├── network/
│   └── filter.rs        Interface filtering logic
├── ui/
│   ├── graphs.rs        RX/TX line chart rendering
│   ├── interface.rs     Interface info table
│   └── help.rs          Help screen
└── utils/
    └── format.rs        Byte/speed formatting utilities
```

## Contributing

Contributions are welcome! Areas for improvement:

- [ ] More graph types (bar charts, histograms)
- [ ] Export statistics to CSV/JSON
- [ ] Packet count tracking
- [ ] Error/drop rate monitoring
- [ ] Customizable update intervals
- [ ] Color theme customization

Please feel free to:
- Report bugs via [GitHub Issues](https://github.com/coder3101/ifmon/issues)
- Submit Pull Requests
- Suggest new features

## Roadmap

- **v0.2.0**: Export functionality and packet statistics
- **v0.3.0**: Configurable themes and layouts
- **v1.0.0**: Stable API with plugin support

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Author

**Ashar Khan** - [coder3101](https://github.com/coder3101)

## Acknowledgments

Built with excellent open-source tools:

- **[ratatui](https://ratatui.rs/)** - Rust library for building rich terminal UIs
- **[netdev](https://crates.io/crates/netdev)** - Cross-platform network interface statistics
- **[crossterm](https://crates.io/crates/crossterm)** - Terminal manipulation library

Inspired by classic tools: `iftop`, `nethogs`, `bmon`

## Similar Projects

- **[bandwhich](https://github.com/imsnif/bandwhich)** - Per-process bandwidth monitor (requires root)
- **[bottom](https://github.com/ClementTsang/bottom)** - System monitor with network stats
- **[zenith](https://github.com/bvaisvil/zenith)** - htop-like system monitor

`ifmon` focuses specifically on interface-level monitoring with zero configuration.

---

**Made with ❤️ by [Ashar Khan](https://github.com/coder3101)**

If you find this useful, consider giving it a ⭐ on [GitHub](https://github.com/coder3101/ifmon)!
