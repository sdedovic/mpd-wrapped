# MPD Wrapped

Giving MPD the year-in-review feature it never needed

## How it Works

`mpd-wrapped` will connect to MPD and listen for any status changes. It keeps track of the current playing song and emits an event when it considers a song played. These events are stored in a SQLite database with (most) metadata for aggregation and analysis. That's it for now, super simple. Everything is local, nothing is sent to / via the internet.

There is a caveat to this approach - the songs are store with the metadata present at listen-time. This means that if you listen to some songs, edit the metadata (tags), and keep tracking, you may have some discrepencies in the tracked songs. For example, if the genre or composer is modified, only new listens will pick up these changes. This descision was made for simplicity.

## Using
```
# connect to MPD and start tracking
mpd-wrapped --listen
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
