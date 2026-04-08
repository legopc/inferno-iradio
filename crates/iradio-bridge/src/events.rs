use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    Vu { slot: usize, l: f32, r: f32 },
    IcyMeta { slot: usize, title: String },
    PlayerUpdate { player: serde_json::Value },
    PlayerStopped { id: String },
    Health { active: usize, max: usize },
    SlotHealth {
        slot: usize,
        connects: u32,
        errors: u32,
        last_error: Option<String>,
    },
}

pub struct EventHub {
    tx: broadcast::Sender<WsEvent>,
}

impl EventHub {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(256);
        EventHub { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.tx.subscribe()
    }

    pub fn sender(&self) -> broadcast::Sender<WsEvent> {
        self.tx.clone()
    }

    pub fn send(&self, event: WsEvent) {
        let _ = self.tx.send(event);
    }
}
