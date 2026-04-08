use crate::config::Config;
use crate::player::PlayerHandle;
use crate::slot_keeper::SlotSender;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;
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
    /// Volume 0.0 – 1.0 (software gain applied before ALSA write)
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn default_volume() -> f32 { 1.0 }

impl PlayerInfo {
    pub fn new(id: Uuid, slot: usize, name: String, url: String) -> Self {
        let ch_base = ((slot - 1) * 2 + 1) as u32;
        Self {
            id,
            slot,
            name,
            url,
            state: PlayerState::Buffering,
            dante_tx_channels: [ch_base, ch_base + 1],
            alsa_device: format!("inferno_iradio_{}", slot),
            started_at: chrono::Utc::now(),
            volume: 1.0,
            error: None,
        }
    }
}

pub struct AppState {
    pub config: Config,
    pub players: RwLock<HashMap<Uuid, (PlayerInfo, PlayerHandle)>>,
    pub favourites: RwLock<Vec<Station>>,
    /// Per-slot volumes (index 0 = slot 1), persisted across restarts.
    pub slot_volumes: RwLock<Vec<f32>>,
    /// One sender per slot (index 0 = slot 1). Used by create_player to hand
    /// the audio path to a new player without opening a new ALSA device.
    pub slot_senders: Vec<SlotSender>,
}

impl AppState {
    pub fn new(config: Config, slot_senders: Vec<SlotSender>) -> Self {
        let favourites = Self::load_favourites(&config.favourites_path);
        let slot_volumes = Self::load_volumes(&config.volumes_path, config.max_players);
        Self {
            config,
            players: RwLock::new(HashMap::new()),
            favourites: RwLock::new(favourites),
            slot_volumes: RwLock::new(slot_volumes),
            slot_senders,
        }
    }

    fn load_favourites(path: &PathBuf) -> Vec<Station> {
        match std::fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    fn load_volumes(path: &PathBuf, max_slots: usize) -> Vec<f32> {
        let saved: Vec<f32> = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        // Ensure length matches max_slots, filling missing slots with 0.7
        let mut v = saved;
        v.resize(max_slots, 0.7);
        v
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

    /// Save per-slot volumes to disk.
    pub async fn save_volumes(&self) {
        let vols = self.slot_volumes.read().await;
        let json = match serde_json::to_string(&*vols) {
            Ok(j) => j,
            Err(e) => { warn!("save_volumes: serialize error: {}", e); return; }
        };
        if let Some(parent) = self.config.volumes_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(&self.config.volumes_path, json) {
            warn!("save_volumes: write error: {}", e);
        }
    }

    /// Get stored volume for a slot (1-based). Returns 0.7 if not set.
    pub async fn get_slot_volume(&self, slot: usize) -> f32 {
        let vols = self.slot_volumes.read().await;
        vols.get(slot - 1).copied().unwrap_or(0.7)
    }

    /// Set and persist volume for a slot (1-based).
    pub async fn set_slot_volume(&self, slot: usize, volume: f32) {
        {
            let mut vols = self.slot_volumes.write().await;
            if let Some(v) = vols.get_mut(slot - 1) {
                *v = volume.clamp(0.0, 1.0);
            }
        }
        self.save_volumes().await;
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
