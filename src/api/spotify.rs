use crate::daemon::state::{PlaylistInfo, TrackInfo};
use anyhow::Result;
use rspotify::{model::SearchType, prelude::*, AuthCodeSpotify, Credentials, OAuth};
use std::collections::HashSet;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::time::{timeout, Duration};
use tracing::info;

async fn wait_for_oauth_callback(auth_url: &str, timeout_secs: u64) -> Result<String> {
    let listener = match TcpListener::bind("127.0.0.1:8888").await {
        Ok(l) => l,
        Err(e) => {
            return Err(anyhow::anyhow!("Cannot bind port 8888: {}", e));
        }
    };

    println!("🌐 Opening browser to authorize...");

    if let Err(e) = open::that(auth_url) {
        println!("⚠️  Could not auto-open browser: {}", e);
        println!("   Please manually open: {}\n", auth_url);
    }

    println!(
        "⏳ Waiting for authorization (timeout: {}s)...",
        timeout_secs
    );

    let result = timeout(Duration::from_secs(timeout_secs), async {
        loop {
            match listener.accept().await {
                Ok((mut stream, _)) => {
                    let mut reader = BufReader::new(&mut stream);
                    let mut request_line = String::new();

                    if reader.read_line(&mut request_line).await.is_ok() {
                        if let Some(code) = parse_code_from_request(&request_line) {
                            let response = build_success_html();
                            let _ = stream.write_all(response.as_bytes()).await;
                            return Ok(code);
                        }
                    }

                    let error_response = build_error_html();
                    let _ = stream.write_all(error_response.as_bytes()).await;
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Accept error: {}", e));
                }
            }
        }
    })
    .await;

    match result {
        Ok(Ok(code)) => Ok(code),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(anyhow::anyhow!("Timeout waiting for authorization")),
    }
}

fn parse_code_from_request(request_line: &str) -> Option<String> {
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let path = parts[1];
    if !path.starts_with("/callback?") && !path.starts_with("/callback/?") {
        return None;
    }

    let query_str = path.split('?').nth(1)?;
    for param in query_str.split('&') {
        if let Some(code) = param.strip_prefix("code=") {
            let code_clean = code.split('&').next().unwrap_or(code);
            return Some(code_clean.to_string());
        }
    }

    None
}

fn build_success_html() -> String {
    "HTTP/1.1 200 OK\r\n\
     Content-Type: text/html; charset=utf-8\r\n\
     Connection: close\r\n\
     \r\n\
     <!DOCTYPE html>\
     <html><head><title>Spotiline - Authorized</title>\
     <style>body{font-family:sans-serif;text-align:center;padding:50px;background:#1DB954;color:white}\
     h1{font-size:3em}p{font-size:1.2em}</style></head>\
     <body><h1>✓ Authorization Complete!</h1>\
     <p>You can close this window and return to your terminal.</p>\
     <p>Spotiline is now connected to Spotify.</p></body></html>"
        .to_string()
}

fn build_error_html() -> String {
    "HTTP/1.1 400 Bad Request\r\n\
     Content-Type: text/html; charset=utf-8\r\n\
     Connection: close\r\n\
     \r\n\
     <!DOCTYPE html>\
     <html><head><title>Spotiline - Error</title>\
     <style>body{font-family:sans-serif;text-align:center;padding:50px;background:#f44336;color:white}</style></head>\
     <body><h1>✗ Invalid Request</h1>\
     <p>Please authorize through the correct link.</p></body></html>"
        .to_string()
}

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
            redirect_uri: "http://127.0.0.1:8888/callback".to_string(),
            scopes,
            ..Default::default()
        };

        let client = AuthCodeSpotify::new(creds, oauth);

        // Try to load token from keyring or config fallback
        let token_str_opt = if let Ok(entry) = keyring::Entry::new("spotiline", "spotify_token") {
            entry.get_password().ok()
        } else {
            None
        }
        .or_else(crate::config::load_token_from_config);

        if let Some(token_str) = token_str_opt {
            if let Ok(token) = serde_json::from_str(&token_str) {
                *client.token.lock().await.unwrap() = Some(token);
                info!("Loaded token from storage");
                return Ok(Self { client });
            }
        }

        println!("\n╔════════════════════════════════════════════════════════╗");
        println!("║         Welcome to Spotiline! 🎵                        ║");
        println!("║  First-time setup: Let's connect to Spotify            ║");
        println!("╚════════════════════════════════════════════════════════╝\n");

        println!("📋 Prerequisites:");
        println!("  1. Spotify Premium account");
        println!("  2. Spotify Developer credentials\n");

        println!("💡 If you don't have credentials yet:");
        println!("  → Visit: https://developer.spotify.com/dashboard");
        println!("  → Create a new app");
        println!("  → Add redirect URI: http://127.0.0.1:8888/callback");
        println!("  → Copy your Client ID and Secret\n");

        // Perform OAuth flow
        let auth_url = client.get_authorize_url(false)?;

        // Try auto-capture first
        match wait_for_oauth_callback(&auth_url, 120).await {
            Ok(code) => {
                println!("✓ Authorization received!");
                client.request_token(&code).await?;
            }
            Err(e) => {
                println!("\n⚠️  Auto-capture failed: {}", e);
                println!("\n╔═══════════════════════════════════════════════════════╗");
                println!("║  Manual Authorization Required                        ║");
                println!("╚═══════════════════════════════════════════════════════╝\n");

                println!("1. Open this URL in your browser:\n   {}\n", auth_url);
                println!("2. Click 'Agree' to authorize Spotiline");
                println!("3. Your browser will show 'This site can't be reached' - THIS IS NORMAL");
                println!("4. Copy the FULL URL from your browser's address bar");
                println!("   Example: http://127.0.0.1:8888/callback?code=AQBx...\n");
                println!("5. Paste that URL here:");

                let mut redirect_url = String::new();
                std::io::stdin().read_line(&mut redirect_url)?;

                let code = client
                    .parse_response_code(redirect_url.trim())
                    .ok_or_else(|| anyhow::anyhow!("Failed to parse redirect URL. Ensure you copied the full URL including 'http://127.0.0.1:8888/callback?code=...'"))?;
                client.request_token(&code).await?;
            }
        }

        // Save token to keyring and file fallback
        if let Some(token) = client.token.lock().await.unwrap().as_ref() {
            let token_str = serde_json::to_string(&token)?;

            // 1. Try keyring
            let mut keyring_saved = false;
            if let Ok(entry) = keyring::Entry::new("spotiline", "spotify_token") {
                if entry.set_password(&token_str).is_ok() {
                    keyring_saved = true;
                }
            }

            // 2. Always write to config/fallback file as well
            let config_saved = crate::config::save_token_to_config(&token_str).is_ok();

            if keyring_saved || config_saved {
                info!(
                    "Saved token to storage (keyring={}, file={})",
                    keyring_saved, config_saved
                );
                println!("✓ Token saved successfully");
            } else {
                return Err(anyhow::anyhow!(
                    "Failed to save token to both system keychain and config file.\n\
                     Please check file permissions for config directory."
                ));
            }
        }

        println!("\n✓ Setup complete! Starting daemon...\n");

        Ok(Self { client })
    }

    pub async fn new_daemon_mode() -> Result<Self> {
        let (client, _) = Self::from_keyring_or_fail()?;

        let token_str_opt = if let Ok(entry) = keyring::Entry::new("spotiline", "spotify_token") {
            entry.get_password().ok()
        } else {
            None
        }
        .or_else(crate::config::load_token_from_config);

        if let Some(token_str) = token_str_opt {
            if let Ok(token) = serde_json::from_str::<rspotify::Token>(&token_str) {
                *client.token.lock().await.unwrap() = Some(token);
                info!("Loaded token from storage (daemon mode)");
                return Ok(Self { client });
            }
        }

        Err(anyhow::anyhow!(
            "No authentication token found in storage.\n\
             Run 'spotiline daemon start' to authenticate."
        ))
    }

    fn from_keyring_or_fail() -> Result<(AuthCodeSpotify, ())> {
        let client_id = Self::get_credential_or_fail("SPOTIFY_CLIENT_ID")?;
        let client_secret = Self::get_credential_or_fail("SPOTIFY_CLIENT_SECRET")?;

        let creds = Credentials::new(&client_id, &client_secret);

        let mut scopes = HashSet::new();
        scopes.insert("user-read-playback-state".to_string());
        scopes.insert("user-modify-playback-state".to_string());
        scopes.insert("user-read-currently-playing".to_string());
        scopes.insert("playlist-read-private".to_string());
        scopes.insert("user-library-read".to_string());

        let oauth = OAuth {
            redirect_uri: "http://127.0.0.1:8888/callback".to_string(),
            scopes,
            ..Default::default()
        };

        Ok((AuthCodeSpotify::new(creds, oauth), ()))
    }

    fn get_credential_or_fail(env_var: &str) -> Result<String> {
        if let Ok(val) = std::env::var(env_var) {
            if !val.trim().is_empty() {
                return Ok(val.trim().to_string());
            }
        }

        if let Ok(entry) = keyring::Entry::new("spotiline", env_var) {
            if let Ok(val) = entry.get_password() {
                if !val.trim().is_empty() {
                    return Ok(val.trim().to_string());
                }
            }
        }

        let creds = crate::config::load_credentials();
        let val = if env_var == "SPOTIFY_CLIENT_ID" {
            creds.client_id
        } else {
            creds.client_secret
        };
        if let Some(v) = val {
            if !v.trim().is_empty() {
                return Ok(v.trim().to_string());
            }
        }

        Err(anyhow::anyhow!(
            "Credential {} not found. Run 'spotiline daemon start' to authenticate.",
            env_var
        ))
    }

    fn get_or_prompt_credential(env_var: &str, prompt: &str) -> Result<String> {
        if let Ok(val) = std::env::var(env_var) {
            return Ok(val);
        }

        if let Ok(entry) = keyring::Entry::new("spotiline", env_var) {
            if let Ok(val) = entry.get_password() {
                if !val.is_empty() {
                    return Ok(val);
                }
            }
        }

        let creds = crate::config::load_credentials();
        if env_var == "SPOTIFY_CLIENT_ID" {
            if let Some(v) = creds.client_id {
                if !v.is_empty() {
                    return Ok(v);
                }
            }
        } else if let Some(v) = creds.client_secret {
            if !v.is_empty() {
                return Ok(v);
            }
        }

        println!("Enter {}: ", prompt);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();

        let mut keyring_saved = false;
        if let Ok(entry) = keyring::Entry::new("spotiline", env_var) {
            if entry.set_password(&input).is_ok() {
                keyring_saved = true;
            }
        }

        let mut config_creds = crate::config::load_credentials();
        if env_var == "SPOTIFY_CLIENT_ID" {
            config_creds.client_id = Some(input.clone());
        } else {
            config_creds.client_secret = Some(input.clone());
        }
        let config_saved = crate::config::save_credentials(&config_creds).is_ok();

        if keyring_saved || config_saved {
            println!("✓ {} saved", prompt);
        } else {
            println!("⚠️  Warning: Could not save {} to storage.", prompt);
        }

        Ok(input)
    }

    pub async fn get_target_device_id(&self) -> Option<String> {
        if let Ok(devices) = self.client.device().await {
            if let Some(active) = devices.iter().find(|d| d.is_active) {
                if let Some(id) = &active.id {
                    return Some(id.clone());
                }
            }
            if let Some(first) = devices.first() {
                if let Some(id) = &first.id {
                    return Some(id.clone());
                }
            }
        }
        None
    }

    pub async fn play(&self) -> Result<()> {
        let dev = self.get_target_device_id().await;
        self.client.resume_playback(dev.as_deref(), None).await?;
        Ok(())
    }

    pub async fn pause(&self) -> Result<()> {
        let dev = self.get_target_device_id().await;
        self.client.pause_playback(dev.as_deref()).await?;
        Ok(())
    }

    pub async fn next(&self) -> Result<()> {
        let dev = self.get_target_device_id().await;
        self.client.next_track(dev.as_deref()).await?;
        Ok(())
    }

    pub async fn previous(&self) -> Result<()> {
        let dev = self.get_target_device_id().await;
        self.client.previous_track(dev.as_deref()).await?;
        Ok(())
    }

    pub async fn seek(&self, position_ms: u32) -> Result<()> {
        use chrono::TimeDelta;
        let delta = TimeDelta::milliseconds(position_ms as i64);
        let dev = self.get_target_device_id().await;
        self.client.seek_track(delta, dev.as_deref()).await?;
        Ok(())
    }

    pub async fn set_volume(&self, volume: u8) -> Result<()> {
        let dev = self.get_target_device_id().await;
        self.client.volume(volume, dev.as_deref()).await?;
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
        let dev = self.get_target_device_id().await;
        self.client
            .add_item_to_queue(PlayableId::Track(track_id), dev.as_deref())
            .await?;
        Ok(())
    }

    pub async fn play_uri(&self, uri: &str) -> Result<()> {
        use rspotify::model::{PlayableId, TrackId};
        let track_id = TrackId::from_id_or_uri(uri)?;
        let dev = self.get_target_device_id().await;
        self.client
            .start_uris_playback([PlayableId::Track(track_id)], dev.as_deref(), None, None)
            .await?;
        Ok(())
    }

    pub fn extract_playable_item_info(
        item: &rspotify::model::PlayableItem,
    ) -> (String, String, String, u64) {
        use rspotify::model::PlayableItem;
        match item {
            PlayableItem::Track(t) => {
                let name = t.name.clone();
                let artist = t
                    .artists
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let uri = t
                    .id
                    .as_ref()
                    .map(|id| format!("spotify:track:{}", id.id()))
                    .unwrap_or_default();
                let dur = t.duration.num_milliseconds().max(0) as u64;
                (name, artist, uri, dur)
            }
            PlayableItem::Episode(e) => {
                let name = e.name.clone();
                let artist = e.show.name.clone();
                let uri = format!("spotify:episode:{}", e.id.id());
                let dur = e.duration.num_milliseconds().max(0) as u64;
                (name, artist, uri, dur)
            }
            PlayableItem::Unknown(val) => {
                let name = val
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                let artist = if let Some(artists) = val.get("artists").and_then(|a| a.as_array()) {
                    artists
                        .iter()
                        .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                } else if let Some(show) = val.get("show").and_then(|s| s.get("name")).and_then(|n| n.as_str()) {
                    show.to_string()
                } else {
                    String::new()
                };
                let uri = val
                    .get("uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                let dur = val.get("duration_ms").and_then(|d| d.as_u64()).unwrap_or(0);
                (name, artist, uri, dur)
            }
        }
    }

    pub async fn get_currently_playing(&self) -> Result<Option<rspotify::model::PlayableItem>> {
        let ctx = self.client.current_user_playing_item().await?;
        Ok(ctx.and_then(|c| c.item))
    }

    pub async fn get_user_playlists(&self, limit: u32) -> Result<Vec<PlaylistInfo>> {
        let page = self
            .client
            .current_user_playlists_manual(Some(limit), None)
            .await?;
        let mut result = Vec::new();
        for p in page.items {
            result.push(PlaylistInfo {
                id: p.id.id().to_string(),
                name: p.name,
                uri: format!("spotify:playlist:{}", p.id.id()),
                total_tracks: p.items.total,
                owner: p
                    .owner
                    .display_name
                    .unwrap_or_else(|| p.owner.id.id().to_string()),
            });
        }
        Ok(result)
    }

    pub async fn get_playlist_tracks(
        &self,
        playlist_id_or_name: &str,
        limit: u32,
    ) -> Result<(String, Vec<TrackInfo>)> {
        use rspotify::model::PlaylistId;

        let trimmed = playlist_id_or_name.trim();
        let (playlist_id, playlist_name) = if let Ok(id) = PlaylistId::from_id_or_uri(trimmed) {
            let static_id = id.into_static();
            let id_str = static_id.id().to_string();
            let name = match self.client.playlist(static_id.clone(), None, None).await {
                Ok(p) => p.name,
                Err(_) => id_str,
            };
            (static_id, name)
        } else {
            let user_playlists = self.get_user_playlists(50).await?;
            let target = trimmed.to_lowercase();
            let matched = user_playlists
                .into_iter()
                .find(|p| p.name.to_lowercase() == target || p.id == trimmed);
            if let Some(p) = matched {
                let id = PlaylistId::from_id(&p.id)?.into_static();
                (id, p.name)
            } else {
                return Err(anyhow::anyhow!(
                    "Playlist '{}' not found in your library.",
                    playlist_id_or_name
                ));
            }
        };

        let page = self
            .client
            .playlist_items_manual(playlist_id, None, None, Some(limit), None)
            .await?;

        let mut tracks = Vec::new();
        for item in page.items {
            if let Some(playable) = item.item {
                let (name, artist, uri, duration_ms) = Self::extract_playable_item_info(&playable);
                if !name.is_empty() {
                    tracks.push(TrackInfo {
                        name,
                        artist,
                        uri,
                        duration_ms,
                    });
                }
            }
        }

        Ok((playlist_name, tracks))
    }

    pub async fn play_context(&self, uri_or_name: &str) -> Result<()> {
        use rspotify::model::{PlayContextId, PlaylistId};

        let trimmed = uri_or_name.trim();
        let playlist_id = if let Ok(id) = PlaylistId::from_id_or_uri(trimmed) {
            id.into_static()
        } else {
            let user_playlists = self.get_user_playlists(50).await?;
            let target = trimmed.to_lowercase();
            let matched = user_playlists
                .into_iter()
                .find(|p| p.name.to_lowercase() == target || p.id == trimmed);
            if let Some(p) = matched {
                PlaylistId::from_id(&p.id)?.into_static()
            } else {
                return Err(anyhow::anyhow!(
                    "Playlist '{}' not found in your library.",
                    uri_or_name
                ));
            }
        };

        let context_id = PlayContextId::Playlist(playlist_id);
        self.client
            .start_context_playback(context_id, None, None, None)
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

        match results {
            rspotify::model::SearchResult::Tracks(track_page) => {
                for track in track_page.items {
                    tracks.push(TrackInfo {
                        name: track.name.clone(),
                        artist: track
                            .artists
                            .iter()
                            .map(|a| a.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                        uri: track
                            .id
                            .map(|id| format!("spotify:track:{}", id.id()))
                            .unwrap_or_default(),
                        duration_ms: track.duration.num_milliseconds().max(0) as u64,
                    });
                }
            }
            rspotify::model::SearchResult::Albums(album_page) => {
                for album in album_page.items {
                    let artist = album
                        .artists
                        .iter()
                        .map(|a| a.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    tracks.push(TrackInfo {
                        name: album.name,
                        artist: if artist.is_empty() {
                            "Album".to_string()
                        } else {
                            format!("Album • {}", artist)
                        },
                        uri: album
                            .id
                            .map(|id| format!("spotify:album:{}", id.id()))
                            .unwrap_or_default(),
                        duration_ms: 0,
                    });
                }
            }
            rspotify::model::SearchResult::Artists(artist_page) => {
                for artist in artist_page.items {
                    tracks.push(TrackInfo {
                        name: artist.name,
                        artist: "Artist".to_string(),
                        uri: format!("spotify:artist:{}", artist.id.id()),
                        duration_ms: 0,
                    });
                }
            }
            rspotify::model::SearchResult::Playlists(playlist_page) => {
                for playlist in playlist_page.items {
                    let owner = playlist
                        .owner
                        .display_name
                        .unwrap_or_else(|| "Spotify".to_string());
                    tracks.push(TrackInfo {
                        name: playlist.name,
                        artist: format!("Playlist • by {}", owner),
                        uri: format!("spotify:playlist:{}", playlist.id.id()),
                        duration_ms: 0,
                    });
                }
            }
            _ => {}
        }

        Ok(tracks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_code_from_valid_request() {
        let request = "GET /callback?code=AQBx123&state=abc HTTP/1.1";
        assert_eq!(
            parse_code_from_request(request),
            Some("AQBx123".to_string())
        );
    }

    #[test]
    fn test_parse_code_from_trailing_slash_request() {
        let request = "GET /callback/?code=AQBx123&state=abc HTTP/1.1";
        assert_eq!(
            parse_code_from_request(request),
            Some("AQBx123".to_string())
        );
    }

    #[test]
    fn test_parse_code_from_invalid_request() {
        assert_eq!(parse_code_from_request("GET / HTTP/1.1"), None);
        assert_eq!(parse_code_from_request("GET /callback HTTP/1.1"), None);
        assert_eq!(parse_code_from_request(""), None);
    }

    #[test]
    fn test_success_html_contains_headers() {
        let html = build_success_html();
        assert!(html.contains("HTTP/1.1 200 OK"));
        assert!(html.contains("Authorization Complete"));
    }

    #[test]
    fn test_extract_playable_item_info_unknown_track() {
        use rspotify::model::PlayableItem;
        let json = serde_json::json!({
            "name": "Gee",
            "artists": [
                {"name": "Girls' Generation"}
            ],
            "duration_ms": 201160,
            "uri": "spotify:track:0t7kjpVLgOYITrSfFCoBEA"
        });
        let item = PlayableItem::Unknown(json);
        let (name, artist, uri, duration) = SpotifyApi::extract_playable_item_info(&item);
        assert_eq!(name, "Gee");
        assert_eq!(artist, "Girls' Generation");
        assert_eq!(uri, "spotify:track:0t7kjpVLgOYITrSfFCoBEA");
        assert_eq!(duration, 201160);
    }

    #[test]
    fn test_extract_playable_item_info_multiple_artists() {
        use rspotify::model::PlayableItem;
        let json = serde_json::json!({
            "name": "Under Pressure",
            "artists": [
                {"name": "Queen"},
                {"name": "David Bowie"}
            ],
            "duration_ms": 248000,
            "uri": "spotify:track:abc123"
        });
        let item = PlayableItem::Unknown(json);
        let (name, artist, uri, duration) = SpotifyApi::extract_playable_item_info(&item);
        assert_eq!(name, "Under Pressure");
        assert_eq!(artist, "Queen, David Bowie");
        assert_eq!(uri, "spotify:track:abc123");
        assert_eq!(duration, 248000);
    }

    #[test]
    fn test_extract_playable_item_info_unknown_episode() {
        use rspotify::model::PlayableItem;
        let json = serde_json::json!({
            "name": "Rust in Production",
            "show": {
                "name": "Software Engineering Daily"
            },
            "duration_ms": 1800000,
            "uri": "spotify:episode:ep123"
        });
        let item = PlayableItem::Unknown(json);
        let (name, artist, uri, duration) = SpotifyApi::extract_playable_item_info(&item);
        assert_eq!(name, "Rust in Production");
        assert_eq!(artist, "Software Engineering Daily");
        assert_eq!(uri, "spotify:episode:ep123");
        assert_eq!(duration, 1800000);
    }
}
