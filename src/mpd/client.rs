use anyhow::{Context, Result};
use mpd::Idle;
use std::net::ToSocketAddrs;
use std::time::Duration;

/// MPD client wrapper with reconnection logic
pub struct MpdClient {
    host: String,
    port: u16,
    client: Option<mpd::Client>,
}

impl MpdClient {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            client: None,
        }
    }

    /// Connect to MPD with retry logic
    pub fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port)
            .to_socket_addrs()
            .context("Failed to resolve MPD address")?
            .next()
            .context("No address resolved")?;

        loop {
            match mpd::Client::connect(addr) {
                Ok(client) => {
                    println!("Connected to MPD at {}:{}", self.host, self.port);
                    self.client = Some(client);
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Failed to connect to MPD: {}. Retrying in 5s...", e);
                    std::thread::sleep(Duration::from_secs(5));
                }
            }
        }
    }

    /// Ensure connection is active, reconnect if needed
    fn ensure_connected(&mut self) -> Result<()> {
        if self.client.is_none() {
            self.connect()?;
        }
        Ok(())
    }

    /// Get the current song, if playing
    pub fn current_song(&mut self) -> Result<Option<mpd::Song>> {
        self.ensure_connected()?;

        let client = self.client.as_mut().unwrap();

        match client.currentsong() {
            Ok(song) => Ok(song),
            Err(e) => {
                eprintln!("Failed to get current song: {}", e);
                self.client = None; // Force reconnect next time
                Err(e.into())
            }
        }
    }

    /// Get the current playback status
    pub fn status(&mut self) -> Result<mpd::Status> {
        self.ensure_connected()?;

        let client = self.client.as_mut().unwrap();

        match client.status() {
            Ok(status) => Ok(status),
            Err(e) => {
                eprintln!("Failed to get status: {}", e);
                self.client = None;
                Err(e.into())
            }
        }
    }

    /// Wait for changes in MPD state
    /// Returns the subsystems that changed
    pub fn wait_for_changes(&mut self) -> Result<Vec<mpd::Subsystem>> {
        self.ensure_connected()?;

        let client = self.client.as_mut().unwrap();

        match client.wait(&[]) {
            Ok(subsystems) => Ok(subsystems),
            Err(e) => {
                eprintln!("Error waiting for changes: {}", e);
                self.client = None;
                Err(e.into())
            }
        }
    }
}
