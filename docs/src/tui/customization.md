# Customization

## Config File

Optional config at:
- **Linux/macOS:** `~/.config/spotiline/config.toml`
- **Windows:** `%APPDATA%\spotiline\config.toml`

If the file doesn't exist, hardcoded defaults are used.

## Example config.toml

```toml
[daemon]
port = 47836
audio_backend = "auto"  # or "alsa", "pulseaudio", "pipewire"

[ui]
color_bg = "#121212"
color_highlight = "#1DB954"
```

## Colors

Hex color values for TUI theme:
- `color_bg`: Background color (default: `#121212` — dark charcoal)
- `color_highlight`: Accent color (default: `#1DB954` — Spotify green)

Restart the TUI to see changes.
