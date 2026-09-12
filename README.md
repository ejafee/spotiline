<div align="center">

<img src="docs/assets/logo.png" alt="Spotiline Logo" width="128" height="128">

# Spotiline

*Because graphical interfaces are overrated*

[![Crates.io](https://img.shields.io/crates/v/spotiline?style=flat-square&logo=rust)](https://crates.io/crates/spotiline)
[![Rust 1.70+](https://img.shields.io/badge/rust-1.70%2B-orange?style=flat-square)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Build Status](https://img.shields.io/github/actions/workflow/status/ejafee/spotiline/ci.yml?branch=main&style=flat-square&logo=github)](https://github.com/ejafee/spotiline/actions)

**The headless, native Spotify client engineered for terminal power users and autonomous AI agents.**

[Installation](#quick-start) • [Documentation](https://ejafee.github.io/spotiline) • [Contributing](#contributing)

</div>

---

## Features

### Human Mode (TUI)
- 24-bit color dashboard with real-time status
- Keyboard-driven navigation (no mouse needed)
- Live search with instant results
- <50MB RAM idle, <100ms launch time

### Agent Mode (CLI)
- Stateless commands for scripting
- `--json` flag for clean, machine-readable output
- Perfect for AI agents and shell automation
- Zero-config pipeline integration

---

## Quick Start

### Prerequisites

1. **Install Rust** (if not already installed):
   ```bash
   # Linux / macOS
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Windows
   # Download from https://rustup.rs or use: winget install Rustlang.Rustup
   ```

2. **Get Spotify Developer Credentials:**
   - Visit [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)
   - Click "Create app"
   - Fill in app name and description (any values)
   - Add Redirect URI: `http://127.0.0.1:8888/callback`
   - Save and copy your **Client ID** and **Client Secret**

### Installation

```bash
cargo install spotiline
```

### First-Time Setup

```bash
spotiline daemon start
```

You will be prompted for:
- Spotify Client ID (paste from dashboard)
- Spotify Client Secret (paste from dashboard)

Your browser opens automatically for authorization. Click "Agree" and you're done.

Credentials are stored securely in your system keychain (Windows Credential Manager, macOS Keychain, Linux Secret Service).

### Launch the UI

```bash
spotiline
```

Or control via CLI:

```bash
spotiline play
spotiline status --json
spotiline search "lofi beats" --play
```

---

## Documentation

Full documentation available at: **[https://ejafee.github.io/spotiline](https://ejafee.github.io/spotiline)**

### Quick Links
- [Installation Guide](https://ejafee.github.io/spotiline/getting_started/installation.html)
- [CLI Command Reference](https://ejafee.github.io/spotiline/cli/commands.html)
- [TUI Keyboard Bindings](https://ejafee.github.io/spotiline/tui/keybindings.html)
- [AI & JSON Outputs](https://ejafee.github.io/spotiline/cli/json.html)

---

## Requirements

### System Requirements
- **Rust 1.70+** (for installation via cargo or building from source)
- Operating System: Linux, macOS, or Windows

### Spotify Requirements
- **Spotify Premium** account (required for playback control via Spotify Web API)
- **Spotify Developer credentials** (Client ID and Client Secret)
  - Create free app at: https://developer.spotify.com/dashboard
  - Set redirect URI: `http://127.0.0.1:8888/callback`
- Active internet connection
- At least one active Spotify device (desktop app, mobile, or speaker)

---

## Architecture

```
spotiline binary
    ├─ TUI (ratatui)          # Terminal dashboard
    └─ CLI (clap)             # Command interface
         ↓
    TCP IPC (127.0.0.1:47836)
         ↓
    Background Daemon
         └─ Spotify Web API (rspotify)
```

---

## Contributing

Contributions welcome! See [CONTRIBUTING.md](https://ejafee.github.io/spotiline/contributing.html) for guidelines.

### Roadmap (v1.1+)
- Synced lyrics integration (lrclib.net)
- Podcast support
- Real-time FFT audio visualizer
- Playlist management

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

## Acknowledgments

Built with:
- [rspotify](https://github.com/ramsayleung/rspotify) - Spotify Web API client
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [clap](https://github.com/clap-rs/clap) - CLI argument parser

---

<div align="center">

Made with ❤️ for the terminal

</div>
