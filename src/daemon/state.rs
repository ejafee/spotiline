use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IPCCommand {
    Play,
    Pause,
    Next,
    Prev,
    Seek(u32),
    Volume(u8),
    Status,
    Queue(String),
    PlayUri(String),
    PlayContext(String),
    Playlists {
        limit: Option<u32>,
    },
    PlaylistTracks {
        playlist: String,
        limit: Option<u32>,
    },
    Search {
        query: String,
        search_type: Option<String>,
        play: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IPCResponse {
    Ok,
    State(PlaybackState),
    SearchResults(Vec<TrackInfo>),
    Playlists(Vec<PlaylistInfo>),
    PlaylistTracks {
        playlist_name: String,
        tracks: Vec<TrackInfo>,
    },
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistInfo {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub total_tracks: u32,
    pub owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    pub state: String,
    pub track: String,
    pub artist: String,
    pub progress_ms: u64,
    pub duration_ms: u64,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            state: "stopped".to_string(),
            track: String::new(),
            artist: String::new(),
            progress_ms: 0,
            duration_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackInfo {
    pub name: String,
    pub artist: String,
    pub uri: String,
    pub duration_ms: u64,
}
