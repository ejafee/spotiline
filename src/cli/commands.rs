use super::{send_command, Commands, DaemonAction};
use crate::daemon::state::IPCCommand;
use anyhow::{anyhow, Result};
use std::process::{Command as ProcessCommand, Stdio};
use tracing::info;

pub async fn execute(command: Commands, port: u16, json_mode: bool) -> Result<()> {
    let res = match command {
        Commands::Daemon { action } => match action {
            DaemonAction::Start => start_daemon(port).await,
            DaemonAction::Stop => stop_daemon().await,
            DaemonAction::Status => daemon_status(port).await,
            DaemonAction::Diagnose => daemon_diagnose(port).await,
        },
        Commands::Play => send_and_print(port, IPCCommand::Play, json_mode).await,
        Commands::Pause => send_and_print(port, IPCCommand::Pause, json_mode).await,
        Commands::Next => send_and_print(port, IPCCommand::Next, json_mode).await,
        Commands::Prev => send_and_print(port, IPCCommand::Prev, json_mode).await,
        Commands::Seek { seconds } => {
            send_and_print(port, IPCCommand::Seek(seconds), json_mode).await
        }
        Commands::Volume { level } => {
            if level > 100 {
                return Err(anyhow!("Volume must be 0-100"));
            }
            send_and_print(port, IPCCommand::Volume(level), json_mode).await
        }
        Commands::Search {
            query,
            r#type,
            play,
        } => {
            send_and_print(
                port,
                IPCCommand::Search {
                    query,
                    search_type: r#type,
                    play,
                },
                json_mode,
            )
            .await
        }
        Commands::Queue { uri } => send_and_print(port, IPCCommand::Queue(uri), json_mode).await,
        Commands::Status => send_and_print(port, IPCCommand::Status, json_mode).await,
    };

    res
}

async fn send_and_print(port: u16, cmd: IPCCommand, json_mode: bool) -> Result<()> {
    match send_command(port, cmd).await {
        Ok(crate::daemon::state::IPCResponse::Error(err)) => {
            let user_msg = friendly_error(&err);
            if json_mode {
                println!(r#"{{"error":"{}"}}"#, user_msg.replace('"', "\\\""));
            } else {
                eprintln!("✗ Error: {}", user_msg);
            }
            std::process::exit(1);
        }
        Ok(resp) => {
            print_response(resp, json_mode);
            Ok(())
        }
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("Connection refused") || err_str.contains("10061") {
                Err(anyhow!(
                    "Daemon not running. Start it with: spotiline daemon start"
                ))
            } else {
                Err(e)
            }
        }
    }
}

fn friendly_error(err: &str) -> String {
    if err.contains("404") || err.contains("NO_ACTIVE_DEVICE") {
        "No active Spotify device found. Please start playback on your phone, desktop, or web player first.".to_string()
    } else if err.contains("403") {
        "Action forbidden. A Spotify Premium account is required for playback control.".to_string()
    } else if err.contains("429") {
        "Rate limited by Spotify API. Please wait a moment and try again.".to_string()
    } else {
        err.to_string()
    }
}

fn print_response(response: crate::daemon::state::IPCResponse, json_mode: bool) {
    use crate::daemon::state::IPCResponse;

    match response {
        IPCResponse::Ok => {
            if json_mode {
                println!(r#"{{"status":"ok"}}"#);
            } else {
                println!("✓ OK");
            }
        }
        IPCResponse::State(state) => {
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&state).unwrap());
            } else {
                println!("State: {}", state.state);
                println!("Track: {}", state.track);
                println!("Artist: {}", state.artist);
                println!(
                    "Progress: {}ms / {}ms",
                    state.progress_ms, state.duration_ms
                );
            }
        }
        IPCResponse::SearchResults(tracks) => {
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&tracks).unwrap());
            } else {
                for (i, track) in tracks.iter().enumerate() {
                    println!(
                        "{}. {} - {} ({})",
                        i + 1,
                        track.name,
                        track.artist,
                        track.uri
                    );
                }
            }
        }
        IPCResponse::Error(err) => {
            if json_mode {
                println!(r#"{{"error":"{}"}}"#, err.replace('"', "\\\""));
            } else {
                eprintln!("✗ Error: {}", err);
            }
        }
    }
}

async fn preflight_auth_check() -> Result<()> {
    use crate::api::SpotifyApi;

    println!("🔍 Checking authentication...");
    let _api = SpotifyApi::new().await?;
    println!("✓ Authentication complete");
    Ok(())
}

fn is_process_running(pid: u32) -> bool {
    #[cfg(windows)]
    {
        ProcessCommand::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output()
            .map(|o| {
                let out = String::from_utf8_lossy(&o.stdout);
                !out.is_empty() && !out.contains("INFO: No tasks") && out.contains(&pid.to_string())
            })
            .unwrap_or(false)
    }

    #[cfg(unix)]
    {
        ProcessCommand::new("ps")
            .args(["-p", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

async fn start_daemon(port: u16) -> Result<()> {
    // Pre-flight authentication check (runs in foreground, shows prompts)
    preflight_auth_check().await?;

    let pid_path = get_pid_path()?;

    if pid_path.exists() {
        let pid_str = std::fs::read_to_string(&pid_path)?;
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            let running = is_process_running(pid);

            if running {
                return Err(anyhow!(
                    "Daemon already running (PID {}). Use 'spotiline daemon stop' to stop it.",
                    pid
                ));
            } else {
                // Stale PID file, remove it
                let _ = std::fs::remove_file(&pid_path);
                info!("Removed stale PID file");
            }
        } else {
            // Invalid PID file, remove it
            let _ = std::fs::remove_file(&pid_path);
        }
    }

    let exe = std::env::current_exe()?;

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

        let child = ProcessCommand::new(exe)
            .arg("--daemon-mode")
            .arg(port.to_string())
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        std::fs::write(&pid_path, child.id().to_string())?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        let child = ProcessCommand::new(exe)
            .arg("--daemon-mode")
            .arg(port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()?;

        std::fs::write(&pid_path, child.id().to_string())?;
    }

    println!("Daemon started");

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    match send_command(port, IPCCommand::Status).await {
        Ok(_) => {
            println!("✓ Daemon is responding");
        }
        Err(e) => {
            let pid_str = std::fs::read_to_string(&pid_path).unwrap_or_default();
            let pid: u32 = pid_str.trim().parse().unwrap_or(0);
            println!("⚠️  Warning: Daemon may have crashed during initialization");
            println!("   Error: {}", e);
            println!(
                "   Hint: Run 'spotiline daemon status' for details or check logs in {:?}",
                get_log_dir().unwrap_or_default()
            );
            if pid != 0 {
                let _ = std::fs::remove_file(&pid_path);
                println!("   Removed stale PID file ({})", pid);
            }
        }
    }

    Ok(())
}

async fn stop_daemon() -> Result<()> {
    let pid_path = get_pid_path()?;

    if !pid_path.exists() {
        println!("Daemon not running");
        println!("Hint: Use 'spotiline daemon start' to start it");
        return Ok(());
    }

    let pid_str = std::fs::read_to_string(&pid_path)?;
    let pid: u32 = pid_str.trim().parse()?;

    #[cfg(windows)]
    {
        ProcessCommand::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()?;
    }

    #[cfg(unix)]
    {
        ProcessCommand::new("kill").arg(pid.to_string()).output()?;
    }

    std::fs::remove_file(&pid_path)?;
    println!("Daemon stopped");
    Ok(())
}

async fn daemon_status(port: u16) -> Result<()> {
    let pid_path = get_pid_path()?;

    if !pid_path.exists() {
        println!("Status: Not running (no PID file)");
        return Ok(());
    }

    let pid_str = std::fs::read_to_string(&pid_path)?;
    let pid: u32 = pid_str.trim().parse()?;

    // Check if process running
    let running = is_process_running(pid);

    if !running {
        println!("Status: Dead (PID file exists but process not found)");
        println!("Hint: Run 'spotiline daemon start' to restart");
        return Ok(());
    }

    // Try to connect
    match send_command(port, IPCCommand::Status).await {
        Ok(_) => {
            println!("Status: Running (PID {})", pid);
            println!("Port: {}", port);

            // Show log location
            if let Ok(log_dir) = get_log_dir() {
                let log_file = std::fs::read_dir(&log_dir)
                    .ok()
                    .and_then(|entries| {
                        let mut logs: Vec<_> = entries
                            .filter_map(|e| e.ok())
                            .map(|e| e.path())
                            .filter(|p| {
                                p.file_name()
                                    .map(|n| n.to_string_lossy().starts_with("spotiline.log"))
                                    .unwrap_or(false)
                            })
                            .collect();
                        logs.sort();
                        logs.pop()
                    })
                    .unwrap_or_else(|| log_dir.join("spotiline.log"));
                println!("Logs: {}", log_file.display());
            }
        }
        Err(_) => {
            println!("Status: Process running but not responding (PID {})", pid);
            println!(
                "Hint: May still be initializing, or try 'spotiline daemon stop' then restart"
            );
        }
    }

    Ok(())
}

async fn daemon_diagnose(port: u16) -> Result<()> {
    println!("=== Spotiline Diagnostic Report ===\n");

    println!("1. Checking credentials in storage...");
    let file_creds = crate::config::load_credentials();

    for key in ["SPOTIFY_CLIENT_ID", "SPOTIFY_CLIENT_SECRET"] {
        let mut found = false;

        // Check keyring
        if let Ok(entry) = keyring::Entry::new("spotiline", key) {
            if let Ok(val) = entry.get_password() {
                if !val.is_empty() {
                    println!("   ✓ {} found in Keyring (length: {})", key, val.len());
                    found = true;
                }
            }
        }

        // Check file fallback
        let file_val = if key == "SPOTIFY_CLIENT_ID" {
            file_creds.client_id.clone()
        } else {
            file_creds.client_secret.clone()
        };

        if let Some(val) = file_val {
            if !val.is_empty() {
                println!("   ✓ {} found in Config File (length: {})", key, val.len());
                found = true;
            }
        }

        if !found {
            println!("   ✗ {} not found in Keyring or Config File", key);
        }
    }

    println!("\n2. Checking authentication token...");
    let mut token_found = false;

    // Check keyring
    if let Ok(entry) = keyring::Entry::new("spotiline", "spotify_token") {
        if let Ok(token_str) = entry.get_password() {
            if !token_str.is_empty() {
                println!("   ✓ Token found in Keyring (length: {})", token_str.len());
                token_found = true;
            }
        }
    }

    // Check config file fallback
    if let Some(token_str) = crate::config::load_token_from_config() {
        if !token_str.is_empty() {
            println!(
                "   ✓ Token found in Config File (length: {})",
                token_str.len()
            );
            token_found = true;
        }
    }

    if !token_found {
        println!("   ✗ Token not found in Keyring or Config File");
    }

    println!("\n3. Checking daemon status...");
    daemon_status(port).await?;

    println!("\n=== End of Diagnostic Report ===");
    Ok(())
}

fn get_log_dir() -> Result<std::path::PathBuf> {
    let log_dir = if cfg!(target_os = "windows") {
        dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find local data dir"))?
            .join("spotiline")
            .join("logs")
    } else if cfg!(target_os = "macos") {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find home dir"))?
            .join("Library")
            .join("Logs")
            .join("spotiline")
    } else {
        dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find local data dir"))?
            .join("spotiline")
    };
    Ok(log_dir)
}

fn get_pid_path() -> Result<std::path::PathBuf> {
    let state_dir = if cfg!(target_os = "windows") {
        dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find local data dir"))?
            .join("spotiline")
    } else if cfg!(target_os = "macos") {
        dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find data dir"))?
            .join("spotiline")
    } else {
        dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find local data dir"))?
            .join("spotiline")
    };

    std::fs::create_dir_all(&state_dir)?;
    Ok(state_dir.join("daemon.pid"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_process_running_current() {
        assert!(is_process_running(std::process::id()));
    }

    #[test]
    fn test_is_process_running_dead() {
        assert!(!is_process_running(999_999_999));
    }

    #[test]
    fn test_friendly_error() {
        assert!(friendly_error("http error: status code 404 Not Found")
            .contains("No active Spotify device"));
        assert!(friendly_error("403 Forbidden").contains("Spotify Premium"));
        assert!(friendly_error("429 Too Many Requests").contains("Rate limited"));
        assert_eq!(friendly_error("Custom error"), "Custom error");
    }
}
