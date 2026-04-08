# inferno-iradio [![v2.0.0](https://img.shields.io/badge/version-2.0.0-blue)](#)

**Internet radio → Dante AoIP bridge with real-time mixer UI**

For the [Inferno appliance](https://github.com/legopc/inferno-aoip-releases).

Streams HTTP/Icecast/SHOUTcast radio to stereo Dante TX channel pairs via the Inferno ALSA PCM plugin. Each concurrent player occupies one Dante TX stereo pair (L+R) and is managed independently.

## Features

- 🎵 Stream any HTTP/Icecast/SHOUTcast URL (MP3, AAC, OGG, FLAC)
- 🔍 Browse & search 30,000+ stations via [RadioBrowser](https://www.radio-browser.info/)
- 📻 1–4 concurrent players, each mapped to a unique Dante TX stereo pair
- 🔀 Slot overwrite — selecting an occupied slot (shown as REPLACE) stops the current stream and starts the new one
- 📊 Real-time VU meters (WebSocket, 20fps) with dBFS levels and peak hold
- 📡 ICY metadata display with scrolling track ticker
- 🎚️ Per-slot gain staging ±6dB with tanh soft-clip limiter
- 🔉 Graceful 50ms fade-out on slot switch (no audio clicks)
- 🩺 Stream health monitoring per slot (connect count, error tracking)
- 🌐 Responsive mixer UI (laptop / tablet / phone)
- ⭐ Persistent presets — star any station in Scan, manage + play from Now Playing
- 💾 Svelte + Vite frontend (compiled and embedded in binary via rust-embed)
- 🔌 REST API — scriptable from Home Assistant or any HTTP client
- 🔐 Optional HTTP Basic Auth
- 🔇 Dante keepalive — TX channels stay announced even when no stream is playing
- 📶 Live connection status — 10s health polling keeps UI accurate if WebSocket drops

## Web UI

The mixer UI has two tabs:

### NOW PLAYING
Displays all configured slots (S1–S4) in fixed positions — empty slots show a greyed placeholder, active slots show a full channel strip with:
- Station name + ICY track ticker
- Real-time 12-segment stereo VU meter with peak hold
- Per-slot volume fader (cubic taper for natural feel)
- ±6dB gain trim input
- State badge (BUFFERING / PLAYING / STOPPED / ERROR)
- Stop button

A collapsible **★ PRESETS** panel below the channel grid allows playing any saved preset into any slot with a single tap.

### SCAN STATIONS
Browse and search 30,000+ stations from RadioBrowser. Each row has:
- ▶ Play — opens slot picker (FREE = empty slot, REPLACE = overwrite existing stream)
- ★ Save/remove preset (filled ★ = already saved)

Selecting a slot that is already occupied stops the running stream before starting the new one.

The **DEFAULT VOL** slider in the header sets the initial volume for all new players. Per-slot faders allow independent adjustment after that.

## Quick Start (Inferno appliance)

The cockpit-inferno application manages channel count and launches the service automatically when `INFERNO_MODE=iradio` is selected. The web UI is reachable at:

```
http://<node-ip>:8765
```

The service is **only active in `iradio` mode** — it is stopped automatically when any other Inferno mode is selected.

## Quick Start (standalone)

```bash
# Install config
cp config.example.toml /etc/iradio/config.toml
# edit max_players, auth, etc. as needed

# Run
iradio-bridge --config /etc/iradio/config.toml

# Open web UI
xdg-open http://localhost:8765
```

## Dante Channel Mapping

Each player slot maps to a consecutive stereo Dante TX pair. The number of active slots is set by `max_players` in config (1–4).

| Player Slot | ALSA Device        | Dante TX Channels | ALT_PORT |
|:-----------:|:------------------:|:-----------------:|:--------:|
| 1           | inferno_iradio_1   | 1–2               | 6100     |
| 2           | inferno_iradio_2   | 3–4               | 6120     |
| 3           | inferno_iradio_3   | 5–6               | 6140     |
| 4           | inferno_iradio_4   | 7–8               | 6160     |

ALSA PCM blocks are written to `~/.asoundrc` automatically on startup when `alsa.setup_alsa = true`. Channel count changes require a service restart.

## REST API

Base URL: `http://<host>:8765/api/v2`

### System

```
GET  /health                       — service health + active player count
GET  /config                       — running config (credentials redacted)
GET  /volume                       — default volume for all slots
PUT  /volume  {"volume": 0.0–1.0}  — set + persist default volume for all slots
```

### Players

```
GET    /players                              — list active players
POST   /players  {"url","name","slot"?,"volume"?}  — start player; omit slot to auto-assign next free;
                                               posting to an occupied slot stops the current stream
GET    /players/:id                          — player info
DELETE /players/:id                          — stop player
PATCH  /players/:id/volume  {"volume": 0–1}  — adjust live volume
PATCH  /players/:id/gain    {"gain_db": -6 to +6}  — gain staging with soft-clip limiter
```

### Stations (RadioBrowser)

```
GET  /stations/search?q=&limit=      — full-text search
GET  /stations/top?limit=            — top stations by click count
GET  /stations/tags                  — all tags
GET  /stations/countries             — all country codes
GET  /stations/by-tag?tag=           — filter by tag
GET  /stations/by-country?country=   — filter by ISO country code
```

### Favourites

```
GET    /favourites          — saved stations
POST   /favourites          — save a station (Station JSON body)
DELETE /favourites/:id      — remove a favourite
```

### Player JSON shape

```json
{
  "id": "uuid",
  "slot": 1,
  "name": "Radio 538",
  "url": "https://...",
  "state": "playing",
  "dante_tx_channels": [1, 2],
  "alsa_device": "inferno_iradio_1",
  "started_at": "2026-04-08T02:06:55Z",
  "volume": 0.7,
  "gain_db": 0.0
}
```

## Building

```bash
# 1. Install ALSA dev headers
sudo apt install pkg-config libasound2-dev   # Debian/Ubuntu
sudo dnf install pkgconfig alsa-lib-devel    # Fedora/RHEL

# 2. Build the Svelte UI
cd web-ui
npm install
npm run build
cd ..

# 3. Build the Rust binary
cargo build --release

# Binary: target/release/iradio-bridge
```

## Configuration

Copy `config.example.toml` and adjust. All fields have sensible defaults.

### Main settings

| Field | Default | Description |
|-------|---------|-------------|
| `port` | `8765` | HTTP port for web UI and API |
| `max_players` | `4` | Concurrent player slots (1–4); each = one Dante TX stereo pair |
| `favourites_path` | `~/.local/share/iradio/favourites.json` | Favourites persistence |
| `volumes_path` | `~/.local/share/iradio/volumes.json` | Per-slot default volume persistence |
| `auth.enabled` | `false` | Enable HTTP Basic Auth |
| `alsa.setup_alsa` | `false` | Auto-write Inferno PCM blocks to `~/.asoundrc` on startup |
| `alsa.sample_rate` | `48000` | PCM sample rate (must match Dante clock domain) |

### Audio & WebSocket configuration

```toml
[audio]
# Gain range: -6 to +6 dB per slot
gain_db = 0.0
# Fade-out time on slot switch (prevents audio clicks)
fade_out_ms = 50
# Soft-clip limiter threshold (dBFS)
limiter_threshold_db = -0.5

[websocket]
# VU meter update frequency (Hz)
vu_fps = 20
```

Environment variable overrides: `IRADIO_CONFIG`, `IRADIO_PORT`, `IRADIO_AUTH_USER`, `IRADIO_AUTH_PASS`.

## WebSocket API

Real-time events for mixer UI, VU meters, and metadata display.

**Connection**: `GET /api/v2/ws` (WebSocket upgrade)

### Event Types

#### Initial Snapshot
Sent immediately after connection. Contains all active players.
```json
{
  "type": "snapshot",
  "players": [{ "id": "uuid", "slot": 0, "name": "...", "state": "playing", ... }, ...]
}
```

#### VU Meter Batch
Sent at `websocket.vu_fps` rate (default 20Hz). Batches all level updates since last send.
```json
{
  "type": "vu_batch",
  "levels": {
    "1": { "l": -12.0, "r": -14.0 },
    "2": { "l": -18.5, "r": -16.2 }
  }
}
```
Values are in dBFS (-∞ to 0.0). Slots are 1-based. Empty slots omitted.

#### ICY Metadata
Sent when stream metadata (track title) is received.
```json
{
  "type": "icy_meta",
  "slot": 1,
  "title": "Artist - Track Name"
}
```

#### Player Update
Sent when player properties change (volume, gain, state, etc).
```json
{
  "type": "player_update",
  "player": { "id": "uuid", "slot": 1, "name": "...", "gain_db": 2.5, ... }
}
```

#### Player Stopped
Sent when a player stops or is deleted.
```json
{
  "type": "player_stopped",
  "id": "uuid"
}
```

#### Health Status
Sent periodically with overall service health.
```json
{
  "type": "health",
  "active": 2,
  "max": 4
}
```

#### Slot Health
Sent when a slot's connection or error state changes.
```json
{
  "type": "slot_health",
  "slot": 1,
  "connects": 5,
  "errors": 1,
  "last_error": "Connection timeout"
}
```
Tracks stream recovery and failures per slot.

## Streaming Notes

- Connections use `connect_timeout = 10s` only. No per-request body timeout is set — live streams run indefinitely.
- `Accept-Encoding: identity` is sent and auto-decompression is disabled; audio data must not be decompressed.
- On stream error the player retries indefinitely with capped exponential backoff (max 30s between attempts).
- Redirect URLs are only adopted as the canonical stream URL after ≥ 64 KB received, preventing pre-roll ad URLs from becoming the reconnect target.
- ICY metadata is stripped inline from the byte stream when `icy-metaint` is present in the response headers.

## Cockpit Integration

The channel count (1–4 slots) is configured in the **cockpit-inferno** application under the `iradio` mode panel. Changes are written to `~/.config/iradio/config.toml` and the service is restarted automatically. The web UI link (`http://<node-ip>:8765`) is shown in the cockpit panel when the mode is active.

## OCI Integration

To include in an Inferno bootc image, add to `Containerfile`:

```dockerfile
COPY --from=iradio-build /target/release/iradio-bridge /usr/local/bin/iradio-bridge
COPY iradio/systemd/iradio-bridge.service /etc/systemd/system/
COPY iradio/config.example.toml /etc/iradio/config.toml
RUN systemctl enable iradio-bridge.service
```

## Licence

MIT — see [LICENSE](LICENSE)

---

Part of the [Inferno AoIP](https://github.com/legopc) project by **lumifaza**.
