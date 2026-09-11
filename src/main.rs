mod api;
mod cli;
mod config;
mod daemon;
mod logging;
mod tui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use tracing::error;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 3 && args[1] == "--daemon-mode" {
        let port: u16 = args[2].parse().unwrap_or(config::DEFAULT_PORT);
        let _ = logging::init_logging();
        let mut daemon = daemon::Daemon::new(port).await?;
        daemon.run().await?;
        return Ok(());
    }

    let config = config::load_config();
    let _ = logging::init_logging();

    let cli = Cli::parse();

    match cli.command {
        None => {
            if let Err(e) = tui::run_tui(config.port).await {
                error!("TUI failed: {}", e);
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(cmd) => {
            let json_mode = matches!(&cmd, Commands::Status { json: true });
            if let Err(e) = cli::handle_cli_command(cmd, config.port, json_mode).await {
                error!("CLI command failed: {}", e);
                if json_mode {
                    println!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "\\\""));
                } else {
                    eprintln!("Error: {}", e);
                }
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
