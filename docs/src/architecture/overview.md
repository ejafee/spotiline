# System Architecture

## High-Level Overview

```
User (Human / AI Agent)
    ↓
spotiline binary
    ├─ No args → TUI (ratatui)
    └─ Args → CLI (clap)
        ↓
    TCP IPC (127.0.0.1:47836)
        ↓
    Background Daemon
        ├─ rspotify → Spotify Web API
        └─ Playback state sync
```

## Components

### 1. CLI Parser (clap)
Routes commands to TUI or CLI mode based on arguments.

### 2. Background Daemon
- Runs detached process
- TCP server on port 47836
- Handles IPC commands (play, pause, status, etc.)
- Uses `rspotify` to communicate with Spotify Web API

### 3. TUI (ratatui + crossterm)
- Connects to daemon via TCP
- Polls status every 1s for real-time updates
- Live search queries daemon

### 4. IPC Protocol
- Serialized via `bincode`
- Command/Response enums:
  - `IPCCommand`: Play, Pause, Next, Seek, Status, Search, Queue
  - `IPCResponse`: Ok, State, SearchResults, Error

## Data Flow Example

```
User presses Space in TUI
  → TUI sends IPCCommand::Play over TCP
  → Daemon receives, calls rspotify.resume_playback()
  → Daemon responds IPCResponse::Ok
  → TUI updates UI state
```
