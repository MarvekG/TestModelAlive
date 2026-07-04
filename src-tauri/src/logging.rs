use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::time::SystemTime as LogSystemTime;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

pub(crate) fn init() -> Result<WorkerGuard, Box<dyn Error + Send + Sync>> {
    let log_dir = log_dir()?;
    std::fs::create_dir_all(&log_dir)?;
    cleanup_old_logs(&log_dir, Duration::from_secs(30 * 24 * 60 * 60))?;

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("TestModelAlive")
        .filename_suffix("txt")
        .build(log_dir)?;
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_timer(LogSystemTime)
                .with_ansi(false),
        )
        .init();

    Ok(guard)
}

fn log_dir() -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or("HOME/USERPROFILE is not set")?;
    Ok(home.join(".TestModelAlive").join("logs"))
}

fn cleanup_old_logs(log_dir: &std::path::Path, max_age: Duration) -> std::io::Result<()> {
    let cutoff = SystemTime::now()
        .checked_sub(max_age)
        .unwrap_or(SystemTime::UNIX_EPOCH);

    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !file_name.starts_with("TestModelAlive.") || !file_name.ends_with(".txt") {
            continue;
        }

        let modified = entry.metadata()?.modified()?;
        if modified < cutoff {
            std::fs::remove_file(path)?;
        }
    }

    Ok(())
}
