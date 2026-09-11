# Command Reference

All commands require the daemon to be running.

## Daemon Management

```bash
spotiline daemon start   # Start background daemon
spotiline daemon stop    # Stop background daemon
```

## Playback Control

```bash
spotiline play           # Resume playback
spotiline pause          # Pause playback
spotiline next           # Skip to next track
spotiline prev           # Previous track
spotiline seek <seconds> # Jump to position
spotiline volume <0-100> # Set volume
```

## Search

```bash
spotiline search "query" [--type track|album|artist|playlist] [--play]
```

Examples:
```bash
spotiline search "lofi beats"
spotiline search "Daft Punk" --type artist
spotiline search "Starboy" --play  # Play first result
```

## Queue

```bash
spotiline queue <spotify:track:...>
```

## Status

```bash
spotiline status        # Human-readable
spotiline status --json # Pure JSON (for AI agents)
```
