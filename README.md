# inferno-iradio

Internet radio → Dante AoIP bridge for the [Inferno appliance](https://github.com/legopc/inferno-aoip-releases).

Streams HTTP/Icecast radio to stereo Dante TX channel pairs via the Inferno ALSA PCM plugin.
Each concurrent player gets its own Dante TX pair (L+R).

## Features

- 🎵 Stream any HTTP/Icecast/SHOUTcast URL (MP3, AAC, OGG, FLAC)
- 🔍 Browse & search 30,000+ stations via [RadioBrowser](https://www.radio-browser.info/)
- 📻 Up to 4 concurrent players, each mapped to a unique Dante TX stereo pair
- ⭐ Persistent favourites
- 🌐 Responsive web UI (matches Inferno design language)
- 🔌 API-first REST API — scriptable from Home Assistant or any HTTP client
- 🔐 Optional HTTP Basic Auth

## Quick Start

```bash
# Install config
sudo cp config.example.toml /etc/iradio/config.toml

# Run
iradio-bridge --config /etc/iradio/config.toml

# Open web UI
xdg-open http://localhost:8765
```

## Dante Channel Mapping

| Player Slot | ALSA Device | Dante TX Channels | ALT_PORT |
|:-----------:|:-----------:|:-----------------:|:--------:|
| 1           | iradio-1    | 1–2               | 6100     |
| 2           | iradio-2    | 3–4               | 6120     |
| 3           | iradio-3    | 5–6               | 6140     |
| 4           | iradio-4    | 7–8               | 6160     |

## REST API

Base: `http://<host>:8765/api/v1`

```
GET  /health                     — service health
GET  /config                     — running config (sanitised)

GET  /players                    — list active players
POST /players                    — start player {"url","name","slot"?}
GET  /players/:id                — player info
DELETE /players/:id              — stop player

GET  /stations/search?q=&limit=  — RadioBrowser search
GET  /stations/top?limit=        — top stations by clicks
GET  /stations/tags              — all tags
GET  /stations/countries         — all countries
GET  /stations/by-tag?tag=       — stations by tag
GET  /stations/by-country?country= — stations by country code

GET  /favourites                 — saved stations
POST /favourites                 — save station (Station JSON body)
DELETE /favourites/:id           — remove favourite
```

## Building

```bash
# Requires ALSA dev headers
sudo apt install pkg-config libasound2-dev   # Debian/Ubuntu
sudo dnf install pkgconfig alsa-lib-devel    # Fedora

cargo build --release
# Binary: target/release/iradio-bridge
```

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
