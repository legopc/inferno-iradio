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
    pub device_prefix: String,
    /// Start of ALT_PORT range (each player uses base + slot*20)
    pub alt_port_base: u16,
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
        Self {
            device_prefix: "iradio".to_string(),
            alt_port_base: 6100,
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
                "device_prefix": self.alsa.device_prefix,
                "alt_port_base": self.alsa.alt_port_base,
                "sample_rate": self.alsa.sample_rate,
                "buffer_frames": self.alsa.buffer_frames,
            },
            "radiobrowser": {
                "request_timeout_secs": self.radiobrowser.request_timeout_secs,
            }
        })
    }
}
