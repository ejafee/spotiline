# Contributing

Thanks for your interest in contributing to Spotiline!

## Code of Conduct

Be respectful, inclusive, and professional in all interactions.

## Development Setup

```bash
git clone https://github.com/ejafee/spotiline.git
cd spotiline
cargo build
cargo test
```

## PR Guidelines

1. **Fork** the repository
2. **Create a branch** for your feature (`git checkout -b feature/my-feature`)
3. **Write tests** for new functionality
4. **Run tests** before committing (`cargo test`)
5. **Format code** with `cargo fmt`
6. **Run clippy** for lints (`cargo clippy`)
7. **Commit** with clear, concise messages
8. **Push** and create a pull request

## Testing

```bash
cargo test --all
```

Acceptance tests (from PRD):
1. Daemon runs 24h without memory leaks
2. `spotiline status --json | jq .` outputs valid JSON
3. TUI space key responds <100ms
4. Network disconnect logs `[ERROR]` with timestamp

## Roadmap (v1.1+)

- Synced lyrics (lrclib.net integration)
- Podcast support
- Real-time FFT visualizer
- Playlist management
- Spotify Free tier support (if possible)

## Questions?

Open an issue on [GitHub](https://github.com/ejafee/spotiline/issues).
