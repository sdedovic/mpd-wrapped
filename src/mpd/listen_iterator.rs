use crate::mpd::status_iterator::SongStatus;
use mpd::Song;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SongListenRecord {
    pub song: Song,
    pub start: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct CurrentListen {
    song: Song,
    start: chrono::DateTime<chrono::Utc>,
    max_elapsed: Duration,
}

pub struct ListenIterator<I> {
    inner: I,
    current_listen: Option<CurrentListen>,
}

impl<I> ListenIterator<I>
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

impl<I> Iterator for ListenIterator<I>
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
