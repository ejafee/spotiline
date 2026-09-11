# Logging & Debugging

## Log Files

Logs written via `tracing` to OS-specific paths:

- **Linux:** `~/.local/state/spotiline/spotiline.log`
- **macOS:** `~/Library/Logs/spotiline/spotiline.log`
- **Windows:** `%LOCALAPPDATA%\spotiline\logs\spotiline.log`

## Log Levels

Default: `INFO`

Override with environment variable:
```bash
export RUST_LOG=debug
spotiline daemon start
```

Levels: `trace`, `debug`, `info`, `warn`, `error`

## Common Issues

### "Daemon not running"
```bash
spotiline daemon start
```

### "Failed to parse redirect URL"
Ensure you paste the **full URL** from the browser after authorizing, including `http://127.0.0.1:8888/callback?code=...`

### Network errors in logs
Check internet connection. Spotiline requires active connection to Spotify servers.

### Playback not working
Ensure:
1. Spotify Premium account
2. At least one active Spotify device (official app or speaker)
3. Run `spotiline status` to verify connection

## Debug Mode

```bash
RUST_LOG=debug spotiline 2>&1 | tee debug.log
```

Captures all debug output to `debug.log`.
