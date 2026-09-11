# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-09-11

### Added
- `spotiline daemon status` command to check daemon health
- `spotiline --version` flag support
- Interactive setup wizard and pre-flight auth check for first-time users
- Stale PID file auto-cleanup when daemon process is dead

### Fixed
- OAuth prompts no longer hidden during `daemon start` (authentication now completes in foreground before daemon detaches)
- Better error messages with actionable hints when daemon is not running

## [1.0.0] - 2026-09-11

### Added
- **Background Daemon:** Persistent background process managing Spotify connection
- **TUI Mode:** Terminal UI with ratatui
  - Real-time playback status display
  - Live search with instant results
  - Keyboard-driven navigation
  - Progress bar and volume gauge
  - Playlist and track browsing
- **CLI Mode:** Command-line interface with clap
  - `daemon start/stop` - Daemon management
  - `play`, `pause`, `next`, `prev` - Playback control
  - `seek <seconds>` - Position seeking
  - `volume <0-100>` - Volume control
  - `search <query>` - Search tracks with `--type` and `--play` flags
  - `queue <uri>` - Add tracks to queue
  - `status` - Get playback status with `--json` flag
- **OAuth Authentication:** Secure token storage via OS keychain
- **TCP IPC:** Local socket communication (port 47836)
- **Logging:** Comprehensive tracing to OS-specific log files
- **Configuration:** Optional `config.toml` for port and color customization
- **Documentation:** Full mdBook site with installation, API reference, and examples

### Technical Details
- Built with Rust 1.70+
- Uses rspotify for Spotify Web API integration
- Supports Linux, macOS, and Windows
- <50MB RAM idle footprint
- <100ms TUI launch time
- JSON output for AI agent integration

### Requirements
- Spotify Premium account
- Active internet connection
- Spotify API credentials (Client ID & Secret)

[1.0.0]: https://github.com/ejafee/spotiline/releases/tag/v1.0.0
