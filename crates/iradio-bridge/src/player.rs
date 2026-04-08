use crate::config::Config;
use crate::slot_keeper::SlotSender;
use crate::state::{PlayerInfo, PlayerState, SharedState};
use crate::stream::SharedBuffer;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::{oneshot, watch};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Handle to a running player task
pub struct PlayerHandle {
    stop_tx: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
    vol_tx: watch::Sender<f32>,
}

impl PlayerHandle {
    pub async fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), task).await;
        }
    }

    pub fn set_volume(&self, volume: f32) {
        let _ = self.vol_tx.send(volume.clamp(0.0, 1.0));
    }
}

/// Spawn a player task for the given slot.
/// `slot_tx` is the sender to the slot's keepalive/ALSA task.
pub fn spawn_player(
    slot: usize,
    name: String,
    url: String,
    state: SharedState,
    config: &Config,
    initial_volume: f32,
    slot_tx: SlotSender,
) -> (Uuid, PlayerInfo, PlayerHandle) {
    let id = Uuid::new_v4();
    let mut info = PlayerInfo::new(id, slot, name.clone(), url.clone());
    info.volume = initial_volume.clamp(0.0, 1.0);

    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let (vol_tx, vol_rx) = watch::channel(initial_volume.clamp(0.0, 1.0));
    let cfg = config.clone();
    let state_clone = state.clone();
    let id_clone = id;

    let task = tokio::spawn(run_player(
        id_clone, slot, url, name, cfg, state_clone, stop_rx, vol_rx, slot_tx,
    ));

    let handle = PlayerHandle {
        stop_tx: Some(stop_tx),
        task: Some(task),
        vol_tx,
    };

    (id, info, handle)
}

async fn run_player(
    id: Uuid,
    slot: usize,
    url: String,
    _name: String,
    cfg: Config,
    state: SharedState,
    stop_rx: oneshot::Receiver<()>,
    vol_rx: watch::Receiver<f32>,
    slot_tx: SlotSender,
) {
    info!("player[{}] slot={} starting: {}", id, slot, url);

    let buffer: SharedBuffer = Arc::new(Mutex::new(VecDeque::new()));
    let client = reqwest::Client::builder()
        // Never auto-decompress — audio streams are raw bytes, not gzip/brotli.
        // reqwest's decompressor will corrupt MP3/AAC data and emit "error decoding
        // response body" if the server sends any Content-Encoding header.
        .no_gzip()
        .no_brotli()
        .no_deflate()
        // Connection-only timeout — does NOT kill an ongoing stream after N seconds.
        // Do NOT use .timeout() here: it sets a total-request deadline which would
        // terminate a live radio stream after N seconds ("error decoding response body").
        .connect_timeout(Duration::from_secs(10))
        .user_agent("VLC/3.0.20 LibVLC/3.0.20")
        .build()
        .unwrap_or_default();

    let (fetch_stop_tx, fetch_stop_rx) = oneshot::channel();
    let _fetch_task = crate::stream::start_stream_fetch(
        url.clone(),
        buffer.clone(),
        client,
        fetch_stop_rx,
    );

    // Wait for initial buffer fill (up to 5 s).
    // The slot keeper keeps ALSA fed with silence during this time.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if buffer.lock().unwrap().len() >= 8 * 1024 {
            break;
        }
        if tokio::time::Instant::now() > deadline {
            warn!("player[{}] buffer fill timeout", id);
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Start the streaming decoder (runs in a blocking thread, maintains symphonia state
    // across the full stream — no more chunk-boundary artifacts).
    let (decode_handle, mut pcm_rx) = crate::decode::start_streaming_decode(
        buffer.clone(),
        cfg.alsa.sample_rate,
        cfg.alsa.buffer_frames as usize,
    );

    set_player_state(&state, id, PlayerState::Playing).await;
    info!("player[{}] playing → slot {}", id, slot);

    let mut stop_rx = stop_rx;

    loop {
        tokio::select! {
            biased;

            // Stop signal from API
            _ = &mut stop_rx => break,

            // Decoded PCM ready
            maybe_samples = pcm_rx.recv() => {
                match maybe_samples {
                    Some(mut samples) => {
                        // Apply software volume
                        let vol = *vol_rx.borrow();
                        if vol < 0.999 {
                            for s in &mut samples {
                                *s = (*s as f32 * vol) as i32;
                            }
                        }
                        // Forward to slot keeper → ALSA
                        if slot_tx.tx.send(samples).await.is_err() {
                            error!("player[{}] slot sender gone", id);
                            break;
                        }
                    }
                    None => {
                        // Decoder stopped (stream ended or error)
                        warn!("player[{}] decoder channel closed", id);
                        break;
                    }
                }
            }
        }
    }

    // Cleanly stop the decoder thread and HTTP fetch
    decode_handle.stop.store(true, Ordering::Relaxed);
    let _ = fetch_stop_tx.send(());
    // Give the decode thread a moment to unblock from blocking_send / Read::read
    tokio::time::sleep(Duration::from_millis(100)).await;

    set_player_state(&state, id, PlayerState::Stopped).await;
    info!("player[{}] stopped", id);
}

async fn set_player_state(state: &SharedState, id: Uuid, new_state: PlayerState) {
    let mut players = state.players.write().await;
    if let Some((info, _)) = players.get_mut(&id) {
        info.state = new_state;
    }
}

