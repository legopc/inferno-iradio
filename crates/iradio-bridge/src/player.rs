use crate::alsa::{device_name, InfernoAlsaDevice};
use crate::config::Config;
use crate::state::{PlayerInfo, PlayerState, SharedState};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Handle to a running player task
pub struct PlayerHandle {
    stop_tx: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
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
}

/// Spawn a player task for the given slot.
/// Returns a PlayerHandle for stopping and the initial PlayerInfo.
pub fn spawn_player(
    slot: usize,
    name: String,
    url: String,
    state: SharedState,
    config: &Config,
) -> (Uuid, PlayerInfo, PlayerHandle) {
    let id = Uuid::new_v4();
    let info = PlayerInfo::new(id, slot, name.clone(), url.clone());

    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let cfg = config.clone();
    let state_clone = state.clone();
    let id_clone = id;

    let task = tokio::spawn(run_player(
        id_clone,
        slot,
        url,
        name,
        cfg,
        state_clone,
        stop_rx,
    ));

    let handle = PlayerHandle {
        stop_tx: Some(stop_tx),
        task: Some(task),
    };

    (id, info, handle)
}

async fn run_player(
    id: Uuid,
    slot: usize,
    url: String,
    name: String,
    cfg: Config,
    state: SharedState,
    stop_rx: oneshot::Receiver<()>,
) {
    info!("player[{}] slot={} starting: {}", id, slot, url);

    let dev_str = device_name(slot);

    let alsa_result = InfernoAlsaDevice::open(&dev_str, cfg.alsa.sample_rate, cfg.alsa.buffer_frames);
    let alsa = match alsa_result {
        Ok(d) => d,
        Err(e) => {
            error!("player[{}] ALSA open failed: {}", id, e);
            set_player_error(&state, id, e.to_string()).await;
            return;
        }
    };

    let buffer: Arc<Mutex<VecDeque<u8>>> = Arc::new(Mutex::new(VecDeque::new()));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(cfg.radiobrowser.request_timeout_secs))
        .build()
        .unwrap_or_default();

    let (fetch_stop_tx, fetch_stop_rx) = oneshot::channel();
    let _fetch_task = crate::stream::start_stream_fetch(
        url.clone(),
        buffer.clone(),
        client,
        fetch_stop_rx,
    );

    // Wait for initial buffer fill (up to 5s)
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        {
            let buf = buffer.lock().unwrap();
            if buf.len() >= 8 * 1024 {
                break;
            }
        }
        if tokio::time::Instant::now() > deadline {
            warn!("player[{}] buffer fill timeout", id);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    set_player_state(&state, id, PlayerState::Playing).await;
    info!("player[{}] playing on ALSA device {}", id, dev_str);

    // Decode + write loop runs in a blocking thread
    let buffer_clone = buffer.clone();
    let sample_rate = cfg.alsa.sample_rate;
    let id_loop = id;

    let mut stop_rx = stop_rx;

    loop {
        // Read a chunk from buffer
        let chunk = crate::stream::read_bytes(&buffer_clone, 64 * 1024);
        if chunk.is_empty() {
            tokio::time::sleep(Duration::from_millis(20)).await;

            // Check for stop signal
            match stop_rx.try_recv() {
                Ok(_) | Err(oneshot::error::TryRecvError::Closed) => break,
                Err(oneshot::error::TryRecvError::Empty) => {}
            }
            continue;
        }

        // Decode in blocking thread
        let cursor = std::io::Cursor::new(chunk);
        let decode_result =
            tokio::task::spawn_blocking(move || {
                crate::decode::decode_to_pcm(Box::new(cursor), sample_rate)
            })
            .await;

        match decode_result {
            Ok(Ok((pcm_samples, _))) => {
                if !pcm_samples.is_empty() {
                    if let Err(e) = alsa.write_frames(&pcm_samples) {
                        error!("player[{}] ALSA write error: {}", id_loop, e);
                        set_player_error(&state, id_loop, e.to_string()).await;
                        break;
                    }
                }
            }
            Ok(Err(e)) => {
                warn!("player[{}] decode error: {}", id_loop, e);
                // Non-fatal — continue with next chunk
            }
            Err(e) => {
                error!("player[{}] decode task panicked: {}", id_loop, e);
                break;
            }
        }

        // Check stop signal
        match stop_rx.try_recv() {
            Ok(_) | Err(oneshot::error::TryRecvError::Closed) => break,
            Err(oneshot::error::TryRecvError::Empty) => {}
        }
    }

    let _ = fetch_stop_tx.send(());
    alsa.drain();
    info!("player[{}] stopped", id);
}

async fn set_player_state(state: &SharedState, id: Uuid, new_state: PlayerState) {
    let mut players = state.players.write().await;
    if let Some((info, _)) = players.get_mut(&id) {
        info.state = new_state;
    }
}

async fn set_player_error(state: &SharedState, id: Uuid, error: String) {
    let mut players = state.players.write().await;
    if let Some((info, _)) = players.get_mut(&id) {
        info.state = PlayerState::Error;
        info.error = Some(error);
    }
}
