use crate::daemon::state::TrackInfo;
use anyhow::Result;
use rspotify::{model::SearchType, prelude::*, AuthCodeSpotify, Credentials, OAuth};
use std::collections::HashSet;
use tracing::info;

pub struct SpotifyApi {
    client: AuthCodeSpotify,
}

impl SpotifyApi {
    pub async fn new() -> Result<Self> {
        let creds = Credentials::new(
            &Self::get_or_prompt_credential("SPOTIFY_CLIENT_ID", "Spotify Client ID")?,
            &Self::get_or_prompt_credential("SPOTIFY_CLIENT_SECRET", "Spotify Client Secret")?,
        );

        let mut scopes = HashSet::new();
        scopes.insert("user-read-playback-state".to_string());
        scopes.insert("user-modify-playback-state".to_string());
        scopes.insert("user-read-currently-playing".to_string());
        scopes.insert("playlist-read-private".to_string());
        scopes.insert("user-library-read".to_string());

        let oauth = OAuth {
            redirect_uri: "http://localhost:8888/callback".to_string(),
            scopes,
            ..Default::default()
        };

        let client = AuthCodeSpotify::new(creds, oauth);

        // Try to load token from keyring
        if let Ok(entry) = keyring::Entry::new("spotiline", "spotify_token") {
            if let Ok(token_str) = entry.get_password() {
                if let Ok(token) = serde_json::from_str(&token_str) {
                    *client.token.lock().await.unwrap() = Some(token);
                    info!("Loaded token from keyring");
                    return Ok(Self { client });
                }
            }
        }

        // Perform OAuth flow
        let auth_url = client.get_authorize_url(false)?;
        println!("\n==> Open this URL in your browser:\n{}\n", auth_url);
        println!("After authorizing, paste the full redirect URL here:");

        let mut redirect_url = String::new();
        std::io::stdin().read_line(&mut redirect_url)?;

        let code = client
            .parse_response_code(&redirect_url.trim())
            .ok_or_else(|| anyhow::anyhow!("Failed to parse redirect URL"))?;
        client.request_token(&code).await?;

        // Save token to keyring
        if let Some(token) = client.token.lock().await.unwrap().as_ref() {
            let token_str = serde_json::to_string(&token)?;
            if let Ok(entry) = keyring::Entry::new("spotiline", "spotify_token") {
                let _ = entry.set_password(&token_str);
            }
            info!("Saved token to keyring");
        }

        Ok(Self { client })
    }

    fn get_or_prompt_credential(env_var: &str, prompt: &str) -> Result<String> {
        if let Ok(val) = std::env::var(env_var) {
            return Ok(val);
        }

        // Try keyring first
        if let Ok(entry) = keyring::Entry::new("spotiline", env_var) {
            if let Ok(val) = entry.get_password() {
                return Ok(val);
            }
        }

        println!("Enter {}: ", prompt);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();

        // Save to keyring
        if let Ok(entry) = keyring::Entry::new("spotiline", env_var) {
            let _ = entry.set_password(&input);
        }

        Ok(input)
    }

    pub async fn play(&self) -> Result<()> {
        self.client.resume_playback(None, None).await?;
        Ok(())
    }

    pub async fn pause(&self) -> Result<()> {
        self.client.pause_playback(None).await?;
        Ok(())
    }

    pub async fn next(&self) -> Result<()> {
        self.client.next_track(None).await?;
        Ok(())
    }

    pub async fn previous(&self) -> Result<()> {
        self.client.previous_track(None).await?;
        Ok(())
    }

    pub async fn seek(&self, position_ms: u32) -> Result<()> {
        use chrono::TimeDelta;
        let delta = TimeDelta::milliseconds(position_ms as i64);
        self.client.seek_track(delta, None).await?;
        Ok(())
    }

    pub async fn set_volume(&self, volume: u8) -> Result<()> {
        self.client.volume(volume, None).await?;
        Ok(())
    }

    pub async fn get_current_playback(
        &self,
    ) -> Result<Option<rspotify::model::CurrentPlaybackContext>> {
        Ok(self.client.current_playback(None, None::<&[_]>).await?)
    }

    pub async fn add_to_queue(&self, uri: &str) -> Result<()> {
        use rspotify::model::{PlayableId, TrackId};
        let track_id = TrackId::from_id_or_uri(uri)?;
        self.client
            .add_item_to_queue(PlayableId::Track(track_id), None)
            .await?;
        Ok(())
    }

    pub async fn play_uri(&self, uri: &str) -> Result<()> {
        use rspotify::model::{PlayableId, TrackId};
        let track_id = TrackId::from_id_or_uri(uri)?;
        self.client
            .start_uris_playback([PlayableId::Track(track_id)], None, None, None)
            .await?;
        Ok(())
    }

    pub async fn search(&self, query: &str, search_type: Option<&str>) -> Result<Vec<TrackInfo>> {
        let stype = match search_type {
            Some("album") => SearchType::Album,
            Some("artist") => SearchType::Artist,
            Some("playlist") => SearchType::Playlist,
            _ => SearchType::Track,
        };

        let results = self
            .client
            .search(query, stype, None, None, Some(10), None)
            .await?;

        let mut tracks = Vec::new();

        if let rspotify::model::SearchResult::Tracks(track_page) = results {
            for track in track_page.items {
                tracks.push(TrackInfo {
                    name: track.name.clone(),
                    artist: track
                        .artists
                        .first()
                        .map(|a| a.name.clone())
                        .unwrap_or_default(),
                    uri: track
                        .id
                        .map(|id| format!("spotify:track:{}", id.id()))
                        .unwrap_or_default(),
                    duration_ms: track.duration.num_milliseconds() as u64,
                });
            }
        }

        Ok(tracks)
    }
}
