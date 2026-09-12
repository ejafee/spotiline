# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2026-09-12

### Fixed
- **Critical Performance Fix:** TUI input lag reduced from 700ms to <50ms
  - Refactored event loop to use non-blocking background status polling
  - Status updates now run in separate task (1 second interval)
  - Keyboard input no longer blocked by IPC calls to daemon

### Changed
- Improved README clarity for new users:
  - Added explicit Rust installation requirement
  - Added step-by-step Spotify Developer credential setup
  - Removed excessive emoji usage for professional tone
  - Clarified authentication flow and credential storage
- Updated documentation (`intro.md`, `installation.md`) with Rust prerequisites

### Technical Details
- TUI event polling reduced from 100ms to 50ms for snappier response
- Background task updates playback state every 1 second via tokio channel
- Main event loop no longer blocks on IPC status queries

## [1.1.4] - 2026-09-12

### Added
- Dual credential and token storage: Keyring + config file fallback (`credentials.toml` and `config.toml`)
- Enhanced `daemon diagnose` to show found status for both Keyring and File storage

### Fixed
- Fixed `client_id: Not present` during OAuth due to Windows Credential Manager save/load failures

## [1.1.3] - 2026-09-12

### Added
- `spotiline daemon diagnose` command to troubleshoot credentials, keyring storage, and daemon health
- Explicit error reporting and verification for system keychain token saves (preventing silent credential loss)
- Daemon-safe API initialization (`new_daemon_mode`) preventing background hangs when tokens are missing

### Fixed
- Fixed critical bug where tokens were silently failing to save to Windows Credential Manager / system keyring
- Fixed daemon hanging silently on startup when unauthenticated

## [1.1.2] - 2026-09-12

### Added
- Automatic OAuth callback capture via local HTTP server (no more manual URL paste)
- Auto-open default browser for authorization
- 120-second timeout with graceful fallback to manual mode
- User-friendly success page in browser after authorization
- Clear error messages and manual fallback instructions

### Fixed
- Confusing "This site can't be reached" error during OAuth (now auto-captured with success page)
- Improved manual authorization instructions for edge cases (firewall, port conflict, headless)

## [1.1.1] - 2026-09-11

### Fixed
- Changed OAuth redirect URI from `http://localhost:8888/callback` to `http://127.0.0.1:8888/callback` (Spotify requires IP address format)

### Migration Note
**For existing users (v1.1.0 or earlier):**

If you already set up Spotiline, update your Spotify Developer Dashboard:

1. Visit https://developer.spotify.com/dashboard
2. Open your Spotiline app settings
3. Update Redirect URI from `http://localhost:8888/callback` to `http://127.0.0.1:8888/callback`
4. Save changes
5. Re-authenticate:
   ```bash
   spotiline daemon stop
   spotiline daemon start
   ```

**New users:** The setup wizard now shows the correct URI automatically.

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
[1.1.1]: https://github.com/ejafee/spotiline/releases/tag/v1.1.1
[1.1.2]: https://github.com/ejafee/spotiline/releases/tag/v1.1.2
[1.2.0]: https://github.com/ejafee/spotiline/releases/tag/v1.2.0
