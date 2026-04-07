use bytes::Bytes;
use futures_util::StreamExt;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, warn};

const RING_BUFFER_CAPACITY: usize = 256 * 1024; // 256 KB
const MAX_RETRIES: u32 = 5;
const RETRY_BACKOFF_BASE_MS: u64 = 500;

/// Shared ring buffer between fetcher and decoder
pub type SharedBuffer = Arc<Mutex<VecDeque<u8>>>;

/// Start a streaming HTTP fetch task. Writes bytes into `buffer`.
/// Retries on EOF/error with exponential backoff.
/// Stops when `stop_rx` is signalled.
pub fn start_stream_fetch(
    url: String,
    buffer: SharedBuffer,
    client: reqwest::Client,
    mut stop_rx: tokio::sync::oneshot::Receiver<()>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut retries = 0u32;

        loop {
            debug!("stream: connecting to {}", url);

            let result = tokio::select! {
                r = fetch_stream(&url, &buffer, &client) => r,
                _ = &mut stop_rx => {
                    debug!("stream: stop requested");
                    return;
                }
            };

            match result {
                Ok(()) => {
                    debug!("stream: EOF, reconnecting");
                }
                Err(e) => {
                    warn!("stream: error ({}), retry {}/{}", e, retries + 1, MAX_RETRIES);
                }
            }

            retries += 1;
            if retries > MAX_RETRIES {
                warn!("stream: max retries exceeded, giving up");
                return;
            }

            let backoff = RETRY_BACKOFF_BASE_MS * (1 << retries.min(6));
            tokio::time::sleep(Duration::from_millis(backoff)).await;
        }
    })
}

async fn fetch_stream(
    url: &str,
    buffer: &SharedBuffer,
    client: &reqwest::Client,
) -> anyhow::Result<()> {
    let response = client
        .get(url)
        .header("Icy-MetaData", "1")
        .send()
        .await?
        .error_for_status()?;

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk: Bytes = chunk?;
        let mut buf = buffer.lock().unwrap();
        // Discard oldest bytes if buffer is full to prevent unbounded growth
        while buf.len() + chunk.len() > RING_BUFFER_CAPACITY {
            buf.pop_front();
        }
        buf.extend(chunk.iter());
    }

    Ok(())
}

/// Read up to `n` bytes from the shared buffer (blocking-style, used from sync decode thread)
pub fn read_bytes(buffer: &SharedBuffer, n: usize) -> Vec<u8> {
    let mut buf = buffer.lock().unwrap();
    let take = n.min(buf.len());
    buf.drain(..take).collect()
}
