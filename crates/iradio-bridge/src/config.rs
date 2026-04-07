use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub port: u16,
    pub max_players: usize,
    pub favourites_path: PathBuf,
    pub auth: AuthConfig,
    pub alsa: AlsaConfig,
    pub radiobrowser: RadioBrowserConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    pub enabled: bool,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AlsaConfig {
    /// If true, auto-write pcm.inferno_iradio_N blocks to asoundrc_path on startup.
    pub setup_alsa: bool,
    /// Path to ~/.asoundrc (or equivalent). Only used when setup_alsa = true.
    pub asoundrc_path: PathBuf,
    pub sample_rate: u32,
    pub buffer_frames: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RadioBrowserConfig {
    /// Leave empty to auto-resolve via DNS
    pub api_url: String,
    pub request_timeout_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 8765,
            max_players: 4,
            favourites_path: PathBuf::from("/var/lib/iradio/favourites.json"),
            auth: AuthConfig::default(),
            alsa: AlsaConfig::default(),
            radiobrowser: RadioBrowserConfig::default(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            username: String::new(),
            password: String::new(),
        }
    }
}

impl Default for AlsaConfig {
    fn default() -> Self {
        // Default asoundrc path: $HOME/.asoundrc (resolved at runtime)
        let asoundrc = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".asoundrc"))
            .unwrap_or_else(|_| PathBuf::from("/var/home/core/.asoundrc"));
        Self {
            setup_alsa: false,
            asoundrc_path: asoundrc,
            sample_rate: 48000,
            buffer_frames: 4096,
        }
    }
}

impl Default for RadioBrowserConfig {
    fn default() -> Self {
        Self {
            api_url: String::new(),
            request_timeout_secs: 10,
        }
    }
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let mut cfg: Config = toml::from_str(&contents)?;

        // Env var overrides
        if let Ok(v) = std::env::var("IRADIO_PORT") {
            cfg.port = v.parse()?;
        }
        if let Ok(v) = std::env::var("IRADIO_AUTH_USER") {
            cfg.auth.username = v;
            cfg.auth.enabled = !cfg.auth.username.is_empty();
        }
        if let Ok(v) = std::env::var("IRADIO_AUTH_PASS") {
            cfg.auth.password = v;
        }

        Ok(cfg)
    }

    /// Return a sanitised copy safe to expose via API (no credentials)
    pub fn sanitised(&self) -> serde_json::Value {
        serde_json::json!({
            "port": self.port,
            "max_players": self.max_players,
            "auth": { "enabled": self.auth.enabled },
            "alsa": {
                "setup_alsa": self.alsa.setup_alsa,
                "asoundrc_path": self.alsa.asoundrc_path.to_string_lossy(),
                "sample_rate": self.alsa.sample_rate,
                "buffer_frames": self.alsa.buffer_frames,
            },
            "radiobrowser": {
                "request_timeout_secs": self.radiobrowser.request_timeout_secs,
            }
        })
    }
}
