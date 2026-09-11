mod commands;

use crate::daemon::state::{IPCCommand, IPCResponse};
use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Parser)]
#[command(name = "spotiline")]
#[command(about = "Headless Spotify client for terminal users and AI agents", long_about = None)]
pub struct Cli {
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
    Status {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum DaemonAction {
    /// Start the background daemon
    Start,
    /// Stop the background daemon
    Stop,
}

pub async fn send_command(port: u16, command: IPCCommand) -> Result<IPCResponse> {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await?;

    let data = bincode::serialize(&command)?;
    stream.write_all(&data).await?;

    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await?;

    let response: IPCResponse = bincode::deserialize(&buf[..n])?;
    Ok(response)
}

pub async fn handle_cli_command(command: Commands, port: u16, json_mode: bool) -> Result<()> {
    commands::execute(command, port, json_mode).await
}
