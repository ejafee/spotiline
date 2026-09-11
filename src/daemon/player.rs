use super::state::{IPCCommand, IPCResponse, PlaybackState};
use crate::api::SpotifyApi;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

#[derive(Clone)]
pub struct PlayerManager {
    state: Arc<Mutex<PlaybackState>>,
    api: Arc<SpotifyApi>,
}

impl PlayerManager {
    pub async fn new() -> Result<Self> {
        let api = Arc::new(SpotifyApi::new().await?);
        let state = Arc::new(Mutex::new(PlaybackState::default()));

        Ok(Self { state, api })
    }

    pub async fn handle_command(&self, command: IPCCommand) -> IPCResponse {
        match command {
            IPCCommand::Play => {
                if let Err(e) = self.api.play().await {
                    error!("Play failed: {}", e);
                    return IPCResponse::Error(e.to_string());
                }
                let mut state = self.state.lock().await;
                state.state = "playing".to_string();
                info!("Playback started");
                IPCResponse::Ok
            }
            IPCCommand::Pause => {
                if let Err(e) = self.api.pause().await {
                    error!("Pause failed: {}", e);
                    return IPCResponse::Error(e.to_string());
                }
                let mut state = self.state.lock().await;
                state.state = "paused".to_string();
                info!("Playback paused");
                IPCResponse::Ok
            }
            IPCCommand::Next => {
                if let Err(e) = self.api.next().await {
                    error!("Next failed: {}", e);
                    return IPCResponse::Error(e.to_string());
                }
                info!("Next track");
                IPCResponse::Ok
            }
            IPCCommand::Prev => {
                if let Err(e) = self.api.previous().await {
                    error!("Previous failed: {}", e);
                    return IPCResponse::Error(e.to_string());
                }
                info!("Previous track");
                IPCResponse::Ok
            }
            IPCCommand::Seek(seconds) => {
                if let Err(e) = self.api.seek(seconds as u32 * 1000).await {
                    error!("Seek failed: {}", e);
                    return IPCResponse::Error(e.to_string());
                }
                info!("Seeked to {}s", seconds);
                IPCResponse::Ok
            }
            IPCCommand::Volume(level) => {
                if let Err(e) = self.api.set_volume(level).await {
                    error!("Volume failed: {}", e);
                    return IPCResponse::Error(e.to_string());
                }
                info!("Volume set to {}", level);
                IPCResponse::Ok
            }
            IPCCommand::Status => match self.api.get_current_playback().await {
                Ok(Some(playback)) => {
                    let (track_name, artist_name, duration) = match &playback.item {
                        Some(item) => {
                            use rspotify::model::PlayableItem;
                            match item {
                                PlayableItem::Track(t) => {
                                    let name = t.name.clone();
                                    let artist = t
                                        .artists
                                        .first()
                                        .map(|a| a.name.clone())
                                        .unwrap_or_default();
                                    let dur = t.duration.num_milliseconds() as u64;
                                    (name, artist, dur)
                                }
                                PlayableItem::Episode(e) => {
                                    let name = e.name.clone();
                                    let artist = e.show.name.clone();
                                    let dur = e.duration.num_milliseconds() as u64;
                                    (name, artist, dur)
                                }
                                _ => (String::new(), String::new(), 0),
                            }
                        }
                        None => (String::new(), String::new(), 0),
                    };

                    let state = PlaybackState {
                        state: if playback.is_playing {
                            "playing"
                        } else {
                            "paused"
                        }
                        .to_string(),
                        track: track_name,
                        artist: artist_name,
                        progress_ms: playback
                            .progress
                            .map(|d| d.num_milliseconds() as u64)
                            .unwrap_or(0),
                        duration_ms: duration,
                    };
                    IPCResponse::State(state)
                }
                Ok(None) => IPCResponse::State(PlaybackState::default()),
                Err(e) => {
                    error!("Status query failed: {}", e);
                    IPCResponse::Error(e.to_string())
                }
            },
            IPCCommand::Queue(uri) => {
                if let Err(e) = self.api.add_to_queue(&uri).await {
                    error!("Queue failed: {}", e);
                    return IPCResponse::Error(e.to_string());
                }
                info!("Added to queue: {}", uri);
                IPCResponse::Ok
            }
            IPCCommand::Search {
                query,
                search_type,
                play,
            } => match self.api.search(&query, search_type.as_deref()).await {
                Ok(tracks) => {
                    if play && !tracks.is_empty() {
                        if let Err(e) = self.api.play_uri(&tracks[0].uri).await {
                            error!("Play search result failed: {}", e);
                            return IPCResponse::Error(e.to_string());
                        }
                    }
                    IPCResponse::SearchResults(tracks)
                }
                Err(e) => {
                    error!("Search failed: {}", e);
                    IPCResponse::Error(e.to_string())
                }
            },
        }
    }
}
