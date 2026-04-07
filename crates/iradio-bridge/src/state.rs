use crate::config::Config;
use crate::player::PlayerHandle;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// A station from RadioBrowser or a custom URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub codec: String,
    #[serde(default)]
    pub bitrate: u32,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favicon: String,
}

/// State of a running player slot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PlayerState {
    Buffering,
    Playing,
    Error,
    Stopped,
}

/// Public player info returned by the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: Uuid,
    pub slot: usize,
    pub name: String,
    pub url: String,
    pub state: PlayerState,
    /// Dante TX channel numbers (1-based), e.g. [1, 2] for slot 1
    pub dante_tx_channels: [u32; 2],
    pub alsa_device: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl PlayerInfo {
    pub fn new(id: Uuid, slot: usize, name: String, url: String, prefix: &str) -> Self {
        let ch_base = ((slot - 1) * 2 + 1) as u32;
        Self {
            id,
            slot,
            name,
            url,
            state: PlayerState::Buffering,
            dante_tx_channels: [ch_base, ch_base + 1],
            alsa_device: format!("{}-{}", prefix, slot),
            started_at: chrono::Utc::now(),
            error: None,
        }
    }
}

pub struct AppState {
    pub config: Config,
    pub players: RwLock<HashMap<Uuid, (PlayerInfo, PlayerHandle)>>,
    pub favourites: RwLock<Vec<Station>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let favourites = Self::load_favourites(&config.favourites_path);
        Self {
            config,
            players: RwLock::new(HashMap::new()),
            favourites: RwLock::new(favourites),
        }
    }

    fn load_favourites(path: &PathBuf) -> Vec<Station> {
        match std::fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    pub async fn save_favourites(&self) -> anyhow::Result<()> {
        let favs = self.favourites.read().await;
        let json = serde_json::to_string_pretty(&*favs)?;
        if let Some(parent) = self.config.favourites_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.config.favourites_path, json)?;
        Ok(())
    }

    /// Returns the next free player slot (1-based), or None if all full
    pub async fn next_free_slot(&self) -> Option<usize> {
        let players = self.players.read().await;
        for slot in 1..=self.config.max_players {
            if !players.values().any(|(info, _)| info.slot == slot) {
                return Some(slot);
            }
        }
        None
    }
}

pub type SharedState = Arc<AppState>;
