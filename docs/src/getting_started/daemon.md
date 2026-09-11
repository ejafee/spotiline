# The Background Daemon

Spotiline runs a background daemon that manages your Spotify connection.

## Starting the Daemon

```bash
spotiline daemon start
```

The daemon:
- Connects to Spotify Web API
- Listens on `127.0.0.1:47836` for IPC commands
- Runs in the background until stopped

## Stopping the Daemon

```bash
spotiline daemon stop
```

## Auto-Start Behavior

When you launch the TUI (`spotiline` with no args), it automatically starts the daemon if it's not running.

## Logs

Daemon logs are written to:
- **Linux:** `~/.local/state/spotiline/spotiline.log`
- **macOS:** `~/Library/Logs/spotiline/spotiline.log`
- **Windows:** `%LOCALAPPDATA%\spotiline\logs\spotiline.log`
