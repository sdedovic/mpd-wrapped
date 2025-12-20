use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;
use tracing::info;

mod mpd;
mod persistence;

use persistence::MusicDb;

pub fn get_db_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", "mpd-wrapped")
        .context("Could not determine project directories")?;

    // Get data directory (for the database)
    let data_dir = proj_dirs.data_dir();
    fs::create_dir_all(data_dir).context("Failed to create data directory")?;

    Ok(data_dir.join("music.db"))
}

pub fn get_config_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", "mpd-wrapped")
        .context("Could not determine project directories")?;

    // Get config directory
    let config_dir = proj_dirs.config_dir();
    fs::create_dir_all(config_dir).context("Failed to create config directory")?;

    Ok(config_dir.join("config.toml"))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let mut pargs = pico_args::Arguments::from_env();
    let listener = pargs.contains("--listener");
    let mpd_address = pargs
        .opt_value_from_str("--mpd")?
        .unwrap_or_else(|| "127.0.0.1:6600".to_string());

    let db_path = get_db_path()?;
    if db_path.exists() {
        info!("found existing db at {db_path:?}");
    } else {
        info!("no existing database found, creating one at {db_path:?}");
    }
    let db = MusicDb::new(db_path.as_path())?;

    if listener {
        info!("Connecting to MPD...");
        let status_iter = mpd::StatusIterator::new(mpd_address)?;
        let listen_iter = mpd::ListenIterator::new(status_iter);

        for listen in listen_iter {
            db.log_play(&listen.into())?;
        }
        info!("Disconnected from MPD")
    }

    Ok(())
}
