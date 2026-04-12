use crate::config::Config;
use crate::events::EventHub;
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
    /// ICY metadata track title (updated live from stream)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icy_title: Option<String>,
    /// Per-slot gain in dB (-6.0 to 6.0), applied in audio chain
    #[serde(default)]
    pub gain_db: f32,
}

fn default_volume() -> f32 {
    1.0
}

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
            icy_title: None,
            gain_db: 0.0,
        }
    }
}

/// Health stats for a player slot
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlotHealth {
    pub slot: usize,
    pub connects: u32,
    pub errors: u32,
    pub last_error: Option<String>,
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
    /// Broadcast hub for WebSocket push events
    pub events: EventHub,
    /// Per-slot health stats (index 0 = slot 1)
    pub slot_health: Vec<tokio::sync::RwLock<SlotHealth>>,
    /// Per-slot gain in dB (index 0 = slot 1), persisted to gains_path
    pub slot_gains: RwLock<Vec<f32>>,
}

impl AppState {
    pub fn new(config: Config, slot_senders: Vec<SlotSender>, events: EventHub) -> Self {
        let favourites = Self::load_favourites(&config.favourites_path);
        let slot_volumes = Self::load_volumes(&config.volumes_path, config.max_players);
        let slot_gains = Self::load_gains(&config.audio.gains_path, config.max_players);
        let slot_health: Vec<_> = (1..=config.max_players)
            .map(|slot| {
                tokio::sync::RwLock::new(SlotHealth {
                    slot,
                    ..Default::default()
                })
            })
            .collect();
        Self {
            config,
            players: RwLock::new(HashMap::new()),
            favourites: RwLock::new(favourites),
            slot_volumes: RwLock::new(slot_volumes),
            slot_senders,
            events,
            slot_health,
            slot_gains: RwLock::new(slot_gains),
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
        // Truncate to max_slots if shrunk, then pad if needed
        let mut v = saved;
        v.truncate(max_slots);
        v.resize(max_slots, 0.7);
        v
    }

    fn load_gains(path: &PathBuf, max_slots: usize) -> Vec<f32> {
        let saved: Vec<f32> = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        // Truncate to max_slots if shrunk, then pad if needed
        let mut v = saved;
        v.truncate(max_slots);
        v.resize(max_slots, 0.0);
        v
    }

    pub async fn add_favourite(&self, station: Station) -> bool {
        let mut favs = self.favourites.write().await;
        if favs.iter().any(|f| f.id == station.id) {
            return false;
        }
        favs.push(station);
        let json = serde_json::to_string_pretty(&*favs).unwrap_or_default();
        drop(favs);
        if let Some(parent) = self.config.favourites_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&self.config.favourites_path, json);
        true
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
            Err(e) => {
                warn!("save_volumes: serialize error: {}", e);
                return;
            }
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

    /// Save per-slot gains to disk.
    pub async fn save_gains(&self) {
        let gains = self.slot_gains.read().await;
        let json = match serde_json::to_string(&*gains) {
            Ok(j) => j,
            Err(e) => {
                warn!("save_gains: serialize error: {}", e);
                return;
            }
        };
        if let Some(parent) = self.config.audio.gains_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(&self.config.audio.gains_path, json) {
            warn!("save_gains: write error: {}", e);
        }
    }

    /// Get stored gain for a slot (1-based). Returns 0.0 if not set.
    pub async fn get_slot_gain(&self, slot: usize) -> f32 {
        let gains = self.slot_gains.read().await;
        gains.get(slot - 1).copied().unwrap_or(0.0)
    }

    /// Set and persist gain for a slot (1-based). Clamps to [-6.0, 6.0].
    pub async fn set_slot_gain(&self, slot: usize, gain_db: f32) {
        {
            let mut gains = self.slot_gains.write().await;
            if let Some(g) = gains.get_mut(slot - 1) {
                *g = gain_db.clamp(-6.0, 6.0);
            }
        }
        self.save_gains().await;
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
