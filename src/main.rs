use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use mpd::{Client, Idle, Song, Subsystem};
use std::fs;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, field::debug, info};

mod persistence;

use persistence::MusicDb;

#[derive(Debug, Clone)]
pub struct SongStatus {
    pub song: Song,
    pub duration: Duration,
    pub elapsed: Duration,
}

struct MpdStatusIterator {
    client: Client,
}

impl MpdStatusIterator {
    pub fn new(host: &str, port: u16) -> Result<Self> {
        let addr = format!("{}:{}", host, port)
            .to_socket_addrs()
            .context("Failed to resolve MPD address")?
            .next()
            .context("No address resolved")?;
        match mpd::Client::connect(addr) {
            Ok(client) => Result::Ok(MpdStatusIterator { client: client }),
            Err(e) => Result::Err(anyhow!("Failed to connect to MPD: {e}")),
        }
    }
}

impl Iterator for MpdStatusIterator {
    type Item = SongStatus;

    fn next(&mut self) -> Option<Self::Item> {
        self.client.idle(&[Subsystem::Player]).ok()?;

        let status = self.client.status().ok()?;
        let elapsed = status.elapsed?;
        let duration = status.duration?;

        let song = self.client.currentsong().ok()??;

        Some(SongStatus {
            duration,
            song,
            elapsed,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SongListenRecord {
    pub song: Song,
    pub start: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct CurrentListen {
    song: Song,
    start: chrono::DateTime<chrono::Utc>,
    max_elapsed: Duration,
}

struct SongListenTracker<I> {
    inner: I,
    current_listen: Option<CurrentListen>,
}

impl<I> SongListenTracker<I>
where
    I: Iterator<Item = SongStatus>,
{
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            current_listen: None,
        }
    }

    fn should_emit(max_elapsed: Duration, total_duration: Duration) -> bool {
        let threshold_time = Duration::from_secs(20);
        let threshold_percentage = 0.6;

        let time_threshold_met = max_elapsed >= threshold_time;
        let percentage_threshold_met = total_duration.as_secs() > 0
            && max_elapsed.as_secs_f64() / total_duration.as_secs_f64() >= threshold_percentage;

        time_threshold_met || percentage_threshold_met
    }

    fn is_restart(elapsed: Duration, max_elapsed: Duration) -> bool {
        let restart_threshold = Duration::from_secs(5);
        elapsed < restart_threshold && max_elapsed >= restart_threshold
    }
}

impl<I> Iterator for SongListenTracker<I>
where
    I: Iterator<Item = SongStatus>,
{
    type Item = SongListenRecord;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let status = self.inner.next()?;

            match self.current_listen.take() {
                None => {
                    // First song
                    self.current_listen = Some(CurrentListen {
                        song: status.song,
                        start: chrono::Utc::now(),
                        max_elapsed: status.elapsed,
                    });
                }
                Some(listen) if listen.song.file != status.song.file => {
                    // Different song - check if we should emit the previous listen
                    let should_emit = Self::should_emit(listen.max_elapsed, status.duration);

                    // Start tracking new song
                    self.current_listen = Some(CurrentListen {
                        song: status.song,
                        start: chrono::Utc::now(),
                        max_elapsed: status.elapsed,
                    });

                    if should_emit {
                        return Some(SongListenRecord {
                            song: listen.song,
                            start: listen.start,
                        });
                    }
                }
                Some(mut listen) => {
                    // Same song
                    if Self::is_restart(status.elapsed, listen.max_elapsed) {
                        // Jumped back to start - emit if threshold met
                        let should_emit = Self::should_emit(listen.max_elapsed, status.duration);

                        // Start new listen of same song
                        self.current_listen = Some(CurrentListen {
                            song: listen.song.clone(),
                            start: chrono::Utc::now(),
                            max_elapsed: status.elapsed,
                        });

                        if should_emit {
                            return Some(SongListenRecord {
                                song: listen.song,
                                start: listen.start,
                            });
                        }
                    } else {
                        // Update max_elapsed if progressing forward
                        if status.elapsed > listen.max_elapsed {
                            listen.max_elapsed = status.elapsed;
                        }
                        self.current_listen = Some(listen);
                    }
                }
            }
        }
    }
}

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

    let mut db: MusicDb;

    let db_path = get_db_path()?;
    let db_path_str = db_path.as_os_str();
    if db_path.exists() {
        info!("found existing db at {db_path_str:?}");
    } else {
        info!("no existing database found, creating one at {db_path_str:?}");
        db = MusicDb::new(db_path.as_path());
    }

    println!("Connecting to MPD...");
    let client = MpdStatusIterator::new("127.0.0.1", 6600).unwrap();
    let tracker = SongListenTracker::new(client);

    for record in tracker {
        info!(
            "Song listened: {:?} (started at {})",
            record.song.title.as_deref().unwrap_or("Unknown"),
            record.start
        );
    }

    Ok(())
}
