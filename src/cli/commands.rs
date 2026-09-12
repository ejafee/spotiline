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
        Commands::Status { json } => {
            send_and_print(port, IPCCommand::Status, json || json_mode).await
        }
    };

    res
}

async fn send_and_print(port: u16, cmd: IPCCommand, json_mode: bool) -> Result<()> {
    match send_command(port, cmd).await {
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

async fn start_daemon(port: u16) -> Result<()> {
    // Pre-flight authentication check (runs in foreground, shows prompts)
    preflight_auth_check().await?;

    let pid_path = get_pid_path()?;

    if pid_path.exists() {
        let pid_str = std::fs::read_to_string(&pid_path)?;
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            // Check if process actually running
            #[cfg(windows)]
            let running = ProcessCommand::new("tasklist")
                .args(["/FI", &format!("PID eq {}", pid), "/NH"])
                .output()
                .map(|o| !o.stdout.is_empty() && o.stdout.len() > 10)
                .unwrap_or(false);

            #[cfg(unix)]
            let running = ProcessCommand::new("ps")
                .args(["-p", &pid.to_string()])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

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
    #[cfg(windows)]
    let running = ProcessCommand::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/NH"])
        .output()
        .map(|o| !o.stdout.is_empty() && o.stdout.len() > 10)
        .unwrap_or(false);

    #[cfg(unix)]
    let running = ProcessCommand::new("ps")
        .args(["-p", &pid.to_string()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

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
                println!("Logs: {}", log_dir.join("spotiline.log").display());
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

    println!("1. Checking credentials in keyring...");
    for key in ["SPOTIFY_CLIENT_ID", "SPOTIFY_CLIENT_SECRET"] {
        match keyring::Entry::new("spotiline", key) {
            Ok(entry) => match entry.get_password() {
                Ok(val) if !val.is_empty() => {
                    println!("   ✓ {} found (length: {})", key, val.len());
                }
                Ok(_) => {
                    println!("   ✗ {} is empty", key);
                }
                Err(e) => {
                    println!("   ✗ {} not found: {}", key, e);
                }
            },
            Err(e) => {
                println!("   ✗ Cannot access keyring for {}: {}", key, e);
            }
        }
    }

    println!("\n2. Checking authentication token...");
    match keyring::Entry::new("spotiline", "spotify_token") {
        Ok(entry) => match entry.get_password() {
            Ok(token_str) if !token_str.is_empty() => {
                println!("   ✓ Token found (length: {})", token_str.len());
                if token_str.len() > 2048 {
                    println!("   ⚠️  Token is very large (>2KB), may exceed manager limits");
                }
            }
            Ok(_) => {
                println!("   ✗ Token is empty");
            }
            Err(e) => {
                println!("   ✗ Token not found: {}", e);
            }
        },
        Err(e) => {
            println!("   ✗ Cannot access keyring: {}", e);
        }
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
