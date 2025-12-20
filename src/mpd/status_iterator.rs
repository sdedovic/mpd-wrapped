use anyhow::{anyhow, Context, Result};
use mpd::{Client, Idle, Song, Subsystem};
use std::net::ToSocketAddrs;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone)]
pub struct SongStatus {
    pub song: Song,
    pub duration: Duration,
    pub elapsed: Duration,
}

pub struct StatusIterator {
    client: Client,
}

impl StatusIterator {
    pub fn new(socket_addr: impl AsRef<str>) -> Result<Self> {
        let addr = socket_addr
            .as_ref()
            .to_socket_addrs()
            .context("Failed to resolve MPD address")?
            .next()
            .context("No address resolved")?;
        match Client::connect(addr) {
            Ok(client) => Ok(StatusIterator { client }),
            Err(e) => Err(anyhow!("Failed to connect to MPD: {e}")),
        }
    }

    fn get_status(&mut self) -> Option<SongStatus> {
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

impl Iterator for StatusIterator {
    type Item = SongStatus;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.client.idle(&[Subsystem::Player]).ok()?;
            if let Some(status) = self.get_status() {
                return Some(status);
            }
        }
    }
}
