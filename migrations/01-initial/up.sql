CREATE TABLE IF NOT EXISTS plays
(
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp    INTEGER NOT NULL,
    title        TEXT,
    artist       TEXT,
    album        TEXT,
    album_artist TEXT,
    date         TEXT
);

CREATE TABLE IF NOT EXISTS plays_other_tags
(
    play_id   INTEGER NOT NULL,
    tag_name  TEXT    NOT NULL,
    tag_value TEXT    NOT NULL,
    FOREIGN KEY (play_id) REFERENCES plays (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_plays_timestamp ON plays (timestamp);
CREATE INDEX IF NOT EXISTS idx_plays_artist ON plays (artist);
CREATE INDEX IF NOT EXISTS idx_plays_album ON plays (album);
CREATE INDEX IF NOT EXISTS idx_plays_album_artist ON plays (album_artist);