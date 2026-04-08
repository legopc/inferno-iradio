# inferno-iradio

Internet radio → Dante AoIP bridge for the [Inferno appliance](https://github.com/legopc/inferno-aoip-releases).

Streams HTTP/Icecast/SHOUTcast radio to stereo Dante TX channel pairs via the Inferno ALSA PCM plugin. Each concurrent player occupies one Dante TX stereo pair (L+R) and is managed independently.

## Features

- 🎵 Stream any HTTP/Icecast/SHOUTcast URL (MP3, AAC, OGG, FLAC)
- 🔍 Browse & search 30,000+ stations via [RadioBrowser](https://www.radio-browser.info/)
- 📻 1–4 concurrent players, each mapped to a unique Dante TX stereo pair
- 🔀 Slot overwrite — playing to an occupied slot stops the current stream automatically
- 🔊 Per-player volume control with logarithmic scaling; default volume persisted across restarts
- 📡 ICY metadata display (station name, track title where available)
- ⭐ Persistent favourites
- 🌐 Industrial broadcast-style web UI served on port 8765
- 🔌 API-first REST API — scriptable from Home Assistant or any HTTP client
- 🔐 Optional HTTP Basic Auth
- 🔇 Dante keepalive — TX channels stay announced even when no stream is playing

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

Base URL: `http://<host>:8765/api/v1`

### System

```
GET  /health                       — service health + active player count
GET  /config                       — running config (credentials redacted)
GET  /volume                       — default volume for all slots {"volume": 0.7, "slots": [...]}
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
  "volume": 0.7
}
```

## Building

```bash
# Requires ALSA dev headers
sudo apt install pkg-config libasound2-dev   # Debian/Ubuntu
sudo dnf install pkgconfig alsa-lib-devel    # Fedora/RHEL

cargo build --release
# Binary: target/release/iradio-bridge
```

## Configuration

Copy `config.example.toml` and adjust. All fields have sensible defaults.

Key fields:

| Field | Default | Description |
|-------|---------|-------------|
| `port` | `8765` | HTTP port for web UI and API |
| `max_players` | `4` | Concurrent player slots (1–4); each = one Dante TX stereo pair |
| `favourites_path` | `~/.local/share/iradio/favourites.json` | Favourites persistence |
| `volumes_path` | `~/.local/share/iradio/volumes.json` | Per-slot default volume persistence |
| `auth.enabled` | `false` | Enable HTTP Basic Auth |
| `alsa.setup_alsa` | `false` | Auto-write Inferno PCM blocks to `~/.asoundrc` on startup |
| `alsa.sample_rate` | `48000` | PCM sample rate (must match Dante clock domain) |

Environment variable overrides: `IRADIO_CONFIG`, `IRADIO_PORT`, `IRADIO_AUTH_USER`, `IRADIO_AUTH_PASS`.

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
