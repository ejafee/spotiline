mod events;
mod ui;

pub use ui::App;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use tracing::{error, info};

pub async fn run_tui(port: u16) -> Result<()> {
    ensure_daemon_running(port).await?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(port).await;

    let result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        error!("TUI error: {}", e);
        return Err(e);
    }

    info!("TUI exited cleanly");
    Ok(())
}

async fn ensure_daemon_running(port: u16) -> Result<()> {
    use tokio::net::TcpStream;

    match TcpStream::connect(format!("127.0.0.1:{}", port)).await {
        Ok(_) => Ok(()),
        Err(_) => {
            info!("Daemon not running, auto-starting...");
            use crate::cli::Commands;
            crate::cli::handle_cli_command(
                Commands::Daemon {
                    action: crate::cli::DaemonAction::Start,
                },
                port,
                false,
            )
            .await?;
            tokio::time::sleep(Duration::from_secs(2)).await;
            Ok(())
        }
    }
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let CEvent::Key(key) = event::read()? {
                if events::handle_key_event(app, key).await? {
                    break;
                }
            }
        }

        app.tick().await;
    }
    Ok(())
}
