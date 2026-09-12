use crate::daemon::state::TrackInfo;
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
}
