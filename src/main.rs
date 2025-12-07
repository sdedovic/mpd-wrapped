mod mpd;

use mpd::MpdClient;
use std::time::Instant;
use tracing::info;

struct ListenSession {
    file_path: String,
    artist: String,
    title: String,
    album: String,
    duration_secs: u64,
    started_at: Instant,
    last_position_secs: u64,
    accumulated_secs: u64,
}

impl ListenSession {
    fn new(song: &::mpd::Song, current_position_secs: u64) -> Option<Self> {
        let duration_secs = song.duration?.as_secs();

        Some(Self {
            file_path: song.file.clone(),
            artist: song
                .tags
                .iter()
                .find(|(k, _)| k == "Artist")
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            title: song.title.clone().unwrap_or_else(|| "Unknown".to_string()),
            album: song
                .tags
                .iter()
                .find(|(k, _)| k == "Album")
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            duration_secs,
            started_at: Instant::now(),
            last_position_secs: current_position_secs,
            accumulated_secs: current_position_secs,
        })
    }

    fn update_position(&mut self, current_position_secs: u64) {
        // If position went backwards, song was probably seeked
        if current_position_secs < self.last_position_secs {
            println!(
                "  [Seek detected: {}s -> {}s]",
                self.last_position_secs, current_position_secs
            );
        } else {
            let delta = current_position_secs - self.last_position_secs;
            self.accumulated_secs += delta;
        }
        self.last_position_secs = current_position_secs;
    }

    fn qualifies_as_listen(&self) -> bool {
        // Minimum 30 seconds long
        if self.duration_secs < 30 {
            return false;
        }

        let percent_played = self.accumulated_secs as f64 / self.duration_secs as f64;

        // Scrobble if: played > 50% OR played > 4 minutes
        percent_played >= 0.5 || self.accumulated_secs >= 240
    }

    fn print_summary(&self) {
        let percent = (self.accumulated_secs as f64 / self.duration_secs as f64) * 100.0;
        let qualifies = if self.qualifies_as_listen() {
            "✓ COUNTED"
        } else {
            "✗ SKIPPED"
        };

        println!("\n{} {} - {}", qualifies, self.artist, self.title);
        println!("  Album: {}", self.album);
        println!(
            "  Played: {}s / {}s ({:.1}%)",
            self.accumulated_secs, self.duration_secs, percent
        );
    }
}

fn main() {
    println!("Connecting to MPD...");

    let mut client = MpdClient::new("127.0.0.1".to_string(), 6600);
    client.connect().expect("Failed to connect to MPD");

    println!("Connected! Monitoring playback...\n");

    let mut current_session: Option<ListenSession> = None;

    loop {
        // // Poll every 5 seconds to track position
        // std::thread::sleep(std::time::Duration::from_secs(5));
        client.wait_for_changes().ok();

        let status = match client.status() {
            Ok(s) => s,
            Err(_) => continue,
        };

        println!("{status:#?}")

        // let current_song = client.current_song().ok().flatten();
        //
        // match status.state {
        //     ::mpd::State::Play => {
        //         if let Some(song) = current_song {
        //             let current_pos = status.elapsed.map(|d| d.as_secs()).unwrap_or(0);
        //
        //             match &mut current_session {
        //                 Some(session) if session.file_path == song.file => {
        //                     // Same song, update position
        //                     session.update_position(current_pos);
        //                 }
        //                 _ => {
        //                     // New song - finalize old session with its last known position
        //                     if let Some(session) = current_session.take() {
        //                         session.print_summary();
        //                     }
        //
        //                     if let Some(new_session) = ListenSession::new(&song, current_pos) {
        //                         println!(
        //                             "\n▶ Started: {} - {}",
        //                             new_session.artist, new_session.title
        //                         );
        //                         current_session = Some(new_session);
        //                     }
        //                 }
        //             }
        //         }
        //     }
        //     ::mpd::State::Pause => {
        //         // Update position when paused
        //         if let Some(session) = &mut current_session {
        //             if let Some(elapsed) = status.elapsed {
        //                 session.update_position(elapsed.as_secs());
        //             }
        //         }
        //     }
        //     ::mpd::State::Stop => {
        //         // Finalize session when stopped
        //         if let Some(session) = current_session.take() {
        //             session.print_summary();
        //         }
        //     }
        // }
    }
}
