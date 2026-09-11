use super::{send_command, Commands, DaemonAction};
use crate::daemon::state::IPCCommand;
use anyhow::{anyhow, Result};
use std::process::{Command as ProcessCommand, Stdio};

pub async fn execute(command: Commands, port: u16, json_mode: bool) -> Result<()> {
    match command {
        Commands::Daemon { action } => match action {
            DaemonAction::Start => start_daemon(port).await?,
            DaemonAction::Stop => stop_daemon().await?,
        },
        Commands::Play => {
            let resp = send_command(port, IPCCommand::Play).await?;
            print_response(resp, json_mode);
        }
        Commands::Pause => {
            let resp = send_command(port, IPCCommand::Pause).await?;
            print_response(resp, json_mode);
        }
        Commands::Next => {
            let resp = send_command(port, IPCCommand::Next).await?;
            print_response(resp, json_mode);
        }
        Commands::Prev => {
            let resp = send_command(port, IPCCommand::Prev).await?;
            print_response(resp, json_mode);
        }
        Commands::Seek { seconds } => {
            let resp = send_command(port, IPCCommand::Seek(seconds)).await?;
            print_response(resp, json_mode);
        }
        Commands::Volume { level } => {
            if level > 100 {
                return Err(anyhow!("Volume must be 0-100"));
            }
            let resp = send_command(port, IPCCommand::Volume(level)).await?;
            print_response(resp, json_mode);
        }
        Commands::Search {
            query,
            r#type,
            play,
        } => {
            let resp = send_command(
                port,
                IPCCommand::Search {
                    query,
                    search_type: r#type,
                    play,
                },
            )
            .await?;
            print_response(resp, json_mode);
        }
        Commands::Queue { uri } => {
            let resp = send_command(port, IPCCommand::Queue(uri)).await?;
            print_response(resp, json_mode);
        }
        Commands::Status { json } => {
            let resp = send_command(port, IPCCommand::Status).await?;
            print_response(resp, json || json_mode);
        }
    }
    Ok(())
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

async fn start_daemon(port: u16) -> Result<()> {
    let pid_path = get_pid_path()?;

    if pid_path.exists() {
        return Err(anyhow!("Daemon already running (PID file exists)"));
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
    Ok(())
}

async fn stop_daemon() -> Result<()> {
    let pid_path = get_pid_path()?;

    if !pid_path.exists() {
        return Err(anyhow!("Daemon not running (no PID file)"));
    }

    let pid_str = std::fs::read_to_string(&pid_path)?;
    let pid: u32 = pid_str.trim().parse()?;

    #[cfg(windows)]
    {
        ProcessCommand::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/F"])
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

fn get_pid_path() -> Result<std::path::PathBuf> {
    let state_dir = if cfg!(target_os = "windows") {
        dirs::data_local_dir()
            .ok_or_else(|| anyhow!("Cannot find local data dir"))?
            .join("spotiline")
    } else if cfg!(target_os = "macos") {
        dirs::data_dir()
            .ok_or_else(|| anyhow!("Cannot find data dir"))?
            .join("spotiline")
    } else {
        dirs::data_local_dir()
            .ok_or_else(|| anyhow!("Cannot find local data dir"))?
            .join("spotiline")
    };

    std::fs::create_dir_all(&state_dir)?;
    Ok(state_dir.join("daemon.pid"))
}
