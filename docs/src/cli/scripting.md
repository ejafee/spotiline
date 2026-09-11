# Shell Scripting Examples

## Bash Alias for Now Playing

```bash
alias now="spotiline status --json | jq -r '.track + \" - \" + .artist'"
```

## Auto-tweet Now Playing

```bash
#!/bin/bash
TRACK=$(spotiline status --json | jq -r '.track')
ARTIST=$(spotiline status --json | jq -r '.artist')
twitter tweet "🎵 Now listening: $TRACK by $ARTIST"
```

## Check if Daemon is Running

```bash
if spotiline status &>/dev/null; then
    echo "Daemon is running"
else
    spotiline daemon start
fi
```

## Play Random Search Result

```bash
spotiline search "chill vibes" --json | jq -r '.[0].uri' | xargs spotiline queue
spotiline next
```

## Loop Through Search Results

```bash
spotiline search "synthwave" --json | jq -r '.[] | .name + " by " + .artist'
```
