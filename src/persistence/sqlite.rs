use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayRecord {
    pub timestamp: i64, // Unix timestamp
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub date: Option<String>,
    pub original_date: Option<String>,
    pub composer: Option<String>,
    pub genres: Vec<String>,
    pub performers: Vec<String>,
}

pub struct MusicDb {
    conn: Connection,
}

impl MusicDb {
    /// Create a new database connection and initialize schema
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS plays (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                title TEXT,
                artist TEXT,
                album TEXT,
                album_artist TEXT,
                date TEXT,
                original_date TEXT,
                composer TEXT
            )",
            [],
        )?;

        // Table for genres (many-to-many relationship)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS play_genres (
                play_id INTEGER NOT NULL,
                genre TEXT NOT NULL,
                FOREIGN KEY (play_id) REFERENCES plays(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Table for performers (many-to-many relationship)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS play_performers (
                play_id INTEGER NOT NULL,
                performer TEXT NOT NULL,
                FOREIGN KEY (play_id) REFERENCES plays(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Indexes for better query performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_plays_timestamp ON plays(timestamp)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_plays_artist ON plays(artist)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_plays_album ON plays(album)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_play_genres_play_id ON play_genres(play_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_play_performers_play_id ON play_performers(play_id)",
            [],
        )?;

        Ok(MusicDb { conn })
    }

    /// Log a play record
    pub fn log_play(&self, record: &PlayRecord) -> Result<i64> {
        // Insert the main play record
        self.conn.execute(
            "INSERT INTO plays (timestamp, title, artist, album, album_artist, date, original_date, composer)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.timestamp,
                record.title,
                record.artist,
                record.album,
                record.album_artist,
                record.date,
                record.original_date,
                record.composer,
            ],
        )?;

        let play_id = self.conn.last_insert_rowid();

        // Insert genres
        for genre in &record.genres {
            self.conn.execute(
                "INSERT INTO play_genres (play_id, genre) VALUES (?1, ?2)",
                params![play_id, genre],
            )?;
        }

        // Insert performers
        for performer in &record.performers {
            self.conn.execute(
                "INSERT INTO play_performers (play_id, performer) VALUES (?1, ?2)",
                params![play_id, performer],
            )?;
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

    /// Get top genres by play count
    pub fn top_genres(&self, limit: usize) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT genre, COUNT(*) as play_count
             FROM play_genres
             GROUP BY genre
             ORDER BY play_count DESC
             LIMIT ?1",
        )?;

        let genres = stmt
            .query_map(params![limit], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>>>()?;

        Ok(genres)
    }

    /// Get play count for a date range
    pub fn plays_in_range(&self, start_ts: i64, end_ts: i64) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM plays WHERE timestamp >= ?1 AND timestamp <= ?2",
            params![start_ts, end_ts],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// Get all plays with full details (including genres and performers)
    pub fn get_plays(&self, limit: Option<usize>) -> Result<Vec<PlayRecord>> {
        let query = if let Some(lim) = limit {
            format!(
                "SELECT id, timestamp, title, artist, album, album_artist, date, original_date, composer
                 FROM plays
                 ORDER BY timestamp DESC
                 LIMIT {}",
                lim
            )
        } else {
            "SELECT id, timestamp, title, artist, album, album_artist, date, original_date, composer
             FROM plays
             ORDER BY timestamp DESC"
                .to_string()
        };

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map([], |row| {
            let play_id: i64 = row.get(0)?;

            Ok((
                play_id,
                PlayRecord {
                    timestamp: row.get(1)?,
                    title: row.get(2)?,
                    artist: row.get(3)?,
                    album: row.get(4)?,
                    album_artist: row.get(5)?,
                    date: row.get(6)?,
                    original_date: row.get(7)?,
                    composer: row.get(8)?,
                    genres: Vec::new(),     // Filled below
                    performers: Vec::new(), // Filled below
                },
            ))
        })?;

        let mut plays = Vec::new();
        for row_result in rows {
            let (play_id, mut play) = row_result?;

            // Fetch genres for this play
            let mut genre_stmt = self
                .conn
                .prepare("SELECT genre FROM play_genres WHERE play_id = ?1")?;
            let genres: Vec<String> = genre_stmt
                .query_map(params![play_id], |row| row.get(0))?
                .collect::<Result<Vec<_>>>()?;
            play.genres = genres;

            // Fetch performers for this play
            let mut performer_stmt = self
                .conn
                .prepare("SELECT performer FROM play_performers WHERE play_id = ?1")?;
            let performers: Vec<String> = performer_stmt
                .query_map(params![play_id], |row| row.get(0))?
                .collect::<Result<Vec<_>>>()?;
            play.performers = performers;

            plays.push(play);
        }

        Ok(plays)
    }

    /// Get total play count
    pub fn total_plays(&self) -> Result<i64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM plays", [], |row| row.get(0))?;

        Ok(count)
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
            original_date: None,
            composer: Some("Test Composer".to_string()),
            genres: vec!["Rock".to_string(), "Alternative".to_string()],
            performers: vec!["Performer 1".to_string(), "Performer 2".to_string()],
        };

        let play_id = db.log_play(&record)?;
        assert!(play_id > 0);

        let total = db.total_plays()?;
        assert_eq!(total, 1);

        let top_artists = db.top_artists(10)?;
        assert_eq!(top_artists.len(), 1);
        assert_eq!(top_artists[0].0, "Test Artist");
        assert_eq!(top_artists[0].1, 1);

        let top_genres = db.top_genres(10)?;
        assert_eq!(top_genres.len(), 2);

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
                original_date: None,
                composer: None,
                genres: vec!["Rock".to_string()],
                performers: vec![],
            };
            db.log_play(&record)?;
        }

        let total = db.total_plays()?;
        assert_eq!(total, 5);

        let top_artists = db.top_artists(10)?;
        assert_eq!(top_artists[0].1, 5);

        Ok(())
    }
}
