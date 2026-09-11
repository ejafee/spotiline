use anyhow::Result;
use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_logging() -> Result<()> {
    let log_dir = get_log_dir()?;
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "spotiline.log");

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(file_appender))
        .init();

    Ok(())
}

fn get_log_dir() -> Result<PathBuf> {
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
