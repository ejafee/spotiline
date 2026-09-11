# AI & JSON Outputs

The `--json` flag outputs strict, machine-parseable JSON with no ANSI colors or extra text.

## Status JSON Schema

```bash
spotiline status --json
```

```json
{
  "state": "playing",
  "track": "Starboy",
  "artist": "The Weeknd",
  "progress_ms": 75000,
  "duration_ms": 230000
}
```

**Fields:**
- `state`: `"playing"`, `"paused"`, or `"stopped"`
- `track`: Track name
- `artist`: Primary artist
- `progress_ms`: Current position (milliseconds)
- `duration_ms`: Total duration (milliseconds)

## Search Results JSON

```bash
spotiline search "lofi" --json
```

```json
[
  {
    "name": "Lofi Study",
    "artist": "Chillhop Music",
    "uri": "spotify:track:...",
    "duration_ms": 180000
  }
]
```

## Error JSON

```json
{
  "error": "Daemon not running (no PID file)"
}
```

## Exit Codes

- `0`: Success
- `1`: Error (check stderr or JSON output)
