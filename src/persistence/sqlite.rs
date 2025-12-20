use crate::mpd::SongListenRecord;
use include_dir::{include_dir, Dir};
use rusqlite::{params, Connection, Result};
use rusqlite_migration::Migrations;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayRecord {
    pub timestamp: i64, // Unix timestamp
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub date: Option<String>,
    pub other_tags: HashMap<String, Vec<String>>,
    pub song_duration_seconds: Option<u64>,
}

impl From<SongListenRecord> for PlayRecord {
    fn from(record: SongListenRecord) -> Self {
        let mut tags_map: HashMap<String, Vec<String>> = HashMap::new();

        for (key, value) in record.song.tags {
            tags_map.entry(key).or_default().push(value);
        }

        // don't really see a reason to track these
        tags_map.remove("duration");
        tags_map.remove("Added");
        tags_map.remove("Format");
        tags_map.remove("Track");
        tags_map.remove("Disc");

        // also remove all keys that are sorting variants, e.g. "AlbumArtistSort"
        tags_map.retain(|key, _value| !key.ends_with("Sort"));

        // pull top-level concepts out
        let tag_title = tags_map.remove("Title").and_then(|mut v| v.pop());
        let tag_artist = tags_map.remove("Artist").and_then(|mut v| v.pop());
        let album = tags_map.remove("Album").and_then(|mut v| v.pop());
        let album_artist = tags_map.remove("AlbumArtist").and_then(|mut v| v.pop());
        let date = tags_map.remove("Date").and_then(|mut v| v.pop());
        let song_duration_seconds = record.song.duration.map(|d| d.as_secs());

        PlayRecord {
            timestamp: record.start.timestamp(),
            title: record.song.title.or(tag_title),
            artist: record.song.artist.or(tag_artist),
            album,
            album_artist,
            date,
            other_tags: tags_map,
            song_duration_seconds,
        }
    }
}

pub struct MusicDb {
    conn: Connection,
}

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations");
static MIGRATIONS: LazyLock<Migrations<'static>> =
    LazyLock::new(|| Migrations::from_directory(&MIGRATIONS_DIR).unwrap());

impl MusicDb {
    /// Create a new database connection and initialize schema
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let mut conn = Connection::open(db_path)?;

        MIGRATIONS.to_latest(&mut conn).unwrap();

        Ok(MusicDb { conn })
    }

    /// Log a play record
    pub fn log_play(&self, record: &PlayRecord) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO plays (timestamp, title, artist, album, album_artist, date, song_duration_seconds)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.timestamp,
                record.title,
                record.artist,
                record.album,
                record.album_artist,
                record.date,
                record.song_duration_seconds
            ],
        )?;
        let play_id = self.conn.last_insert_rowid();

        for (tag_name, tag_values) in &record.other_tags {
            for tag_value in tag_values {
                self.conn.execute(
                    "INSERT INTO plays_other_tags (play_id, tag_name, tag_value) values (?1, ?2, ?3)",
                    params![play_id, tag_name, tag_value],
                )?;
            }
        }

        Ok(play_id)
    }

    /// Get top artists by play count
    pub fn top_artists(&self, limit: usize) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT artist, COUNT(*) as play_count
             FROM plays
             WHERE artist IS NOT NULL
             GROUP BY artist
             ORDER BY play_count DESC
             LIMIT ?1",
        )?;

        let artists = stmt
            .query_map(params![limit], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>>>()?;

        Ok(artists)
    }

    /// Get top albums by play count
    pub fn top_albums(&self, limit: usize) -> Result<Vec<(String, String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT album, COALESCE(album_artist, artist) as artist, COUNT(*) as play_count
             FROM plays
             WHERE album IS NOT NULL
             GROUP BY album, artist
             ORDER BY play_count DESC
             LIMIT ?1",
        )?;

        let albums = stmt
            .query_map(params![limit], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(albums)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() -> Result<()> {
        let db = MusicDb::new(":memory:")?;

        let record = PlayRecord {
            timestamp: 1702800000,
            title: Some("Test Song".to_string()),
            artist: Some("Test Artist".to_string()),
            album: Some("Test Album".to_string()),
            album_artist: Some("Test Artist".to_string()),
            date: Some("2023".to_string()),
            other_tags: Default::default(),
            song_duration_seconds: None,
        };

        let play_id = db.log_play(&record)?;
        assert!(play_id > 0);

        let top_artists = db.top_artists(10)?;
        assert_eq!(top_artists.len(), 1);
        assert_eq!(top_artists[0].0, "Test Artist");
        assert_eq!(top_artists[0].1, 1);

        Ok(())
    }

    #[test]
    fn test_multiple_plays() -> Result<()> {
        let db = MusicDb::new(":memory:")?;

        for i in 0..5 {
            let record = PlayRecord {
                timestamp: 1702800000 + i,
                title: Some(format!("Song {}", i)),
                artist: Some("Same Artist".to_string()),
                album: Some("Same Album".to_string()),
                album_artist: Some("Same Artist".to_string()),
                date: Some("2023".to_string()),
                other_tags: Default::default(),
                song_duration_seconds: None,
            };
            db.log_play(&record)?;
        }

        let top_artists = db.top_artists(10)?;
        assert_eq!(top_artists[0].1, 5);

        Ok(())
    }
}
