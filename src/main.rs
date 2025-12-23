use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;
use tracing::info;
use crate::persistence::MusicDb;

mod mpd;
mod persistence;

use crate::persistence::sqlite::TimeInterval;

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

fn print_stats(db: &MusicDb, interval: TimeInterval) -> Result<()> {
    let interval_name = match interval {
        TimeInterval::Week => "Week",
        TimeInterval::Month => "Month",
        TimeInterval::Year => "Year",
        TimeInterval::AllTime => "All Time",
    };

    println!("\n=== Top Artists ({}) ===", interval_name);
    let artists = db.get_top_artists(interval)?;
    for (i, artist) in artists.iter().take(10).enumerate() {
        println!(
            "{}. {} - {} minutes ({} plays)",
            i + 1,
            artist.artist_name,
            artist.total_minutes.round() as i64,
            artist.play_count
        );
    }

    println!("\n=== Top Songs ({}) ===", interval_name);
    let songs = db.get_top_songs(interval)?;
    for (i, song) in songs.iter().take(10).enumerate() {
        println!(
            "{}. {} by {} - {} minutes ({} plays)",
            i + 1,
            song.title,
            song.artist_name,
            song.total_minutes.round() as i64,
            song.play_count
        );
    }

    println!("\n=== Top Albums ({}) ===", interval_name);
    let albums = db.get_top_albums(interval)?;
    for (i, album) in albums.iter().take(10).enumerate() {
        println!(
            "{}. {} by {} - {} minutes ({} plays)",
            i + 1,
            album.album,
            album.artist_name,
            album.total_minutes.round() as i64,
            album.play_count
        );
    }

    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let mut pargs = pico_args::Arguments::from_env();

    // Check for subcommands
    let subcommand: Option<String> = pargs.subcommand()?;

    let db_path = get_db_path()?;
    if db_path.exists() {
        info!("found existing db at {db_path:?}");
    } else {
        info!("no existing database found, creating one at {db_path:?}");
    }
    let db = MusicDb::new(db_path.as_path())?;

    match subcommand.as_deref() {
        Some("query") => {
            let interval = if pargs.contains("--week") {
                TimeInterval::Week
            } else if pargs.contains("--month") {
                TimeInterval::Month
            } else if pargs.contains("--year") {
                TimeInterval::Year
            } else if pargs.contains("--all") {
                TimeInterval::AllTime
            } else {
                // Default to all time if no flag specified
                TimeInterval::AllTime
            };

            print_stats(&db, interval)?;
        }
        Some("listener") => {
            let mpd_address = pargs
                .opt_value_from_str("--mpd")?
                .unwrap_or_else(|| "127.0.0.1:6600".to_string());

            info!("Connecting to MPD...");
            let status_iter = mpd::StatusIterator::new(mpd_address)?;
            let listen_iter = mpd::ListenIterator::new(status_iter);

            for listen in listen_iter {
                db.log_play(&listen.into())?;
            }
            info!("Disconnected from MPD")
        }
        _ => {
            eprintln!("Usage:");
            eprintln!("  mpd-wrapped listener [--mpd <address>]  # Run listener mode");
            eprintln!("  mpd-wrapped query [--week|--month|--year|--all]  # Query statistics");
            eprintln!("\nExamples:");
            eprintln!("  mpd-wrapped query --week");
            eprintln!("  mpd-wrapped query --all");
            eprintln!("  mpd-wrapped listener --mpd 127.0.0.1:6600");
        }
    }

    Ok(())
}