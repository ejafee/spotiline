# Installation

## Prerequisites

Spotiline is written in Rust. You need Rust 1.70 or later installed.

### Install Rust

**Linux / macOS:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows:**
- Download from [rustup.rs](https://rustup.rs)
- Or use winget: `winget install Rustlang.Rustup`

**Verify installation:**
```bash
rustc --version
cargo --version
```

## Via Cargo (Recommended)

```bash
cargo install spotiline
```

## From Source

```bash
git clone https://github.com/ejafee/spotiline.git
cd spotiline
cargo build --release
# Binary at target/release/spotiline
```

## Pre-compiled Binaries

Download from [GitHub Releases](https://github.com/ejafee/spotiline/releases/latest):

- `spotiline-x86_64-unknown-linux-gnu` (Linux x64)
- `spotiline-aarch64-unknown-linux-gnu` (Linux ARM)
- `spotiline-x86_64-apple-darwin` (macOS Intel)
- `spotiline-aarch64-apple-darwin` (macOS Apple Silicon)
- `spotiline-x86_64-pc-windows-msvc.exe` (Windows x64)

## Verify Installation

```bash
spotiline --version
```
