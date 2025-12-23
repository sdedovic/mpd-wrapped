# MPD Wrapped

Giving MPD the year-in-review feature it never needed

## How it Works

`mpd-wrapped` will connect to MPD and listen for any status changes. It keeps track of the current playing song and emits an event when it considers a song played. These events are stored in a SQLite database with (most) metadata for aggregation and analysis. That's it for now, super simple. Everything is local, nothing is sent to / via the internet.

There is a caveat to this approach - the songs are store with the metadata present at listen-time. This means that if you listen to some songs, edit the metadata (tags), and keep tracking, you may have some discrepancies in the tracked songs. For example, if the genre or composer is modified, only new listens will pick up these changes. This descision was made for simplicity.

The SQLite database is stored in your system's data directory:
- Linux: `~/.local/share/mpd-wrapped/music.db`
- macOS: `~/Library/Application Support/mpd-wrapped/music.db`
- Windows: `C:\Users\<User>\AppData\Roaming\mpd-wrapped\music.db`

## Installation
```bash
cargo install --path .
```

## Usage
### Listener Mode
Run the listener to start tracking your MPD playback:
```bash
# Connect to MPD on default address (127.0.0.1:6600)
mpd-wrapped listener

# Connect to MPD on custom address
mpd-wrapped listener --mpd 192.168.1.100:6600
```

The listener will continuously monitor MPD and log each completed song play to the database.

### Query Statistics
Query your listening statistics for different time periods:
```bash
# Last week's stats
mpd-wrapped query --week

# Last month's stats
mpd-wrapped query --month

# Last year's stats
mpd-wrapped query --year

# All-time stats (default)
mpd-wrapped query --all
mpd-wrapped query
```

## Example Output
```
=== Top Artists (Week) ===
1. System of a Down - 42 minutes (14 plays)
2. Radiohead - 34 minutes (7 plays)
3. Reverend Kristin Michael Hayter - 21 minutes (6 plays)
4. Green Day - 18 minutes (6 plays)
5. Zhe Nhir - 13 minutes (3 plays)
6. SkinnyTrips - 13 minutes (2 plays)
7. Kylesa - 8 minutes (2 plays)

=== Top Songs (Week) ===
1. The Riverbed by SkinnyTrips - 7 minutes (1 plays)
2. I WILL BE WITH YOU ALWAYS by Reverend Kristin Michael Hayter - 7 minutes (1 plays)
3. How to Disappear Completely by Radiohead - 6 minutes (1 plays)
4. The National Anthem by Radiohead - 6 minutes (1 plays)
5. Optimistic by Radiohead - 5 minutes (1 plays)
6. Confrontation by SkinnyTrips - 5 minutes (1 plays)
7. Crusher by Kylesa - 5 minutes (1 plays)
8. Bandarlog by Zhe Nhir - 5 minutes (1 plays)
9. Kid A by Radiohead - 5 minutes (1 plays)
10. ALL OF MY FRIENDS ARE GOING TO HELL by Reverend Kristin Michael Hayter - 5 minutes (1 plays)

=== Top Albums (Week) ===
1. Toxicity by System of a Down - 42 minutes (14 plays)
2. KID A MNESIA by Radiohead - 30 minutes (6 plays)
3. SAVED! by Reverend Kristin Michael Hayter - 21 minutes (6 plays)
4. Dookie by Green Day - 18 minutes (6 plays)
5. Piezo by Zhe Nhir - 13 minutes (3 plays)
6. Confrontation by SkinnyTrips - 13 minutes (2 plays)
7. Exhausting Fire by Kylesa - 8 minutes (2 plays)
8. In Rainbows by Radiohead - 4 minutes (1 plays)
```

### Systemd User Service
This is an example, and different values for After, Wants, and ExecStart may be needed.
```
[Unit]
Description=MPD Wrapped
Documentation=
After=network.target mpd.service
Wants=mpd.service

[Service]
Type=simple
ExecStart=%h/.local/bin/mpd-wrapped --listener
Restart=on-failure
RestartSec=5s
StartLimitBurst=3
StartLimitIntervalSec=300

[Install]
WantedBy=default.target
```

## Building / Development
```
# install deps via nix
flake develop

# compile
cargo build --release
```
