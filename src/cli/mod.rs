mod commands;

use crate::daemon::state::{IPCCommand, IPCResponse};
use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Parser)]
#[command(name = "spotiline")]
#[command(version)]
#[command(about = "Headless Spotify client for terminal users and AI agents", long_about = None)]
pub struct Cli {
    /// Format output as JSON (for agents and scripting)
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage the background daemon
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    /// Resume playback
    Play,
    /// Pause playback
    Pause,
    /// Skip to next track
    Next,
    /// Go to previous track
    Prev,
    /// Seek to position in seconds
    Seek { seconds: u32 },
    /// Set volume (0-100)
    Volume { level: u8 },
    /// Search for tracks
    Search {
        query: String,
        #[arg(long)]
        r#type: Option<String>,
        #[arg(long)]
        play: bool,
    },
    /// Add track to queue
    Queue { uri: String },
    /// Get current playback status
    Status,
    /// View and play Spotify playlists
    #[command(alias = "playlists")]
    Playlist {
        #[command(subcommand)]
        action: PlaylistAction,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum PlaylistAction {
    /// List your Spotify playlists
    List {
        #[arg(long, default_value = "50")]
        limit: u32,
    },
    /// View songs in a playlist (by name, ID, or URI)
    Tracks {
        /// Playlist name, Spotify URI, or ID
        playlist: String,
        #[arg(long, default_value = "50")]
        limit: u32,
    },
    /// Play a playlist (by name, ID, or URI)
    Play {
        /// Playlist name, Spotify URI, or ID
        playlist: String,
    },
}

#[derive(Subcommand)]
pub enum DaemonAction {
    /// Start the background daemon
    Start,
    /// Stop the background daemon
    Stop,
    /// Check daemon status
    Status,
    /// Check authentication and keyring status
    Diagnose,
}

pub async fn send_command(port: u16, command: IPCCommand) -> Result<IPCResponse> {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await?;

    let data = bincode::serialize(&command)?;
    let len_bytes = (data.len() as u32).to_be_bytes();
    stream.write_all(&len_bytes).await?;
    stream.write_all(&data).await?;

    let mut resp_len_bytes = [0u8; 4];
    stream.read_exact(&mut resp_len_bytes).await?;
    let resp_len = u32::from_be_bytes(resp_len_bytes) as usize;
    if resp_len > 10 * 1024 * 1024 {
        return Err(anyhow::anyhow!("Response too large: {} bytes", resp_len));
    }

    let mut resp_buf = vec![0u8; resp_len];
    stream.read_exact(&mut resp_buf).await?;

    let response: IPCResponse = bincode::deserialize(&resp_buf)?;
    Ok(response)
}

pub async fn handle_cli_command(command: Commands, port: u16, json_mode: bool) -> Result<()> {
    commands::execute(command, port, json_mode).await
}
