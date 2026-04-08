//! Per-slot ALSA keepalive.
//!
//! Each slot owns its ALSA device permanently. When no player is active the
//! keeper writes silence so the Dante TX channel stays registered in Dante
//! Controller. When a player is active it sends decoded PCM frames via the
//! `SlotSender` and the keeper writes those instead.

use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::alsa::{device_name, InfernoAlsaDevice};
use crate::config::Config;

/// Clone-able handle the player uses to push PCM into the slot's ALSA device.
#[derive(Clone)]
pub struct SlotSender {
    pub tx: mpsc::Sender<Vec<i32>>,
}

/// Start one keepalive task per slot (slots are 1-based).
/// Returns senders indexed 0 … max_slots-1 (index 0 = slot 1).
pub fn start_slot_keepers(max_slots: usize, cfg: &Config) -> Vec<SlotSender> {
    (1..=max_slots)
        .map(|slot| {
            let (tx, rx) = mpsc::channel::<Vec<i32>>(8);
            let cfg2 = cfg.clone();
            tokio::spawn(run_keeper(slot, rx, cfg2));
            SlotSender { tx }
        })
        .collect()
}

async fn run_keeper(slot: usize, mut rx: mpsc::Receiver<Vec<i32>>, cfg: Config) {
    let dev_str = device_name(slot);

    // Retry until the Dante plugin is ready (can take a few seconds after service start).
    let alsa = loop {
        match InfernoAlsaDevice::open(&dev_str, cfg.alsa.sample_rate, cfg.alsa.buffer_frames) {
            Ok(d) => break d,
            Err(e) => {
                warn!("slot {} ALSA open failed: {}, retrying in 3s", slot, e);
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    };
    info!("slot {} keepalive active on {}", slot, dev_str);

    loop {
        match rx.try_recv() {
            Ok(samples) => {
                // Player audio — write it directly.
                let _ = alsa.write_frames(&samples);
            }
            Err(mpsc::error::TryRecvError::Empty) => {
                // Nothing queued — feed silence to keep Dante TX channel alive.
                alsa.write_silence();
                // Yield so other tasks get a turn (write_silence may have blocked
                // ~85 ms waiting for ALSA buffer space, so this is low-overhead).
                tokio::task::yield_now().await;
            }
            Err(mpsc::error::TryRecvError::Disconnected) => {
                warn!("slot {} keepalive: sender disconnected, stopping", slot);
                break;
            }
        }
    }
}
