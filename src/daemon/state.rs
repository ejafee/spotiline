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
    Error(String),
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
