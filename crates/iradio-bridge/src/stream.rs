use bytes::Bytes;
use futures_util::StreamExt;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, warn};

const RING_BUFFER_CAPACITY: usize = 512 * 1024; // 512 KB — larger buffer for smoother decode
const RETRY_BACKOFF_BASE_MS: u64 = 500;
const RETRY_BACKOFF_MAX_MS: u64 = 10_000;

/// Shared ring buffer between fetcher and decoder
pub type SharedBuffer = Arc<Mutex<VecDeque<u8>>>;

/// Start a streaming HTTP fetch task. Writes bytes into `buffer`.
/// Retries indefinitely on EOF/error with exponential backoff (capped at 10 s).
/// Stops when `stop_rx` is signalled.
pub fn start_stream_fetch(
    url: String,
    buffer: SharedBuffer,
    client: reqwest::Client,
    mut stop_rx: tokio::sync::oneshot::Receiver<()>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut attempt = 0u32;

        loop {
            debug!("stream: connecting to {} (attempt {})", url, attempt + 1);

            let result = tokio::select! {
                r = fetch_stream(&url, &buffer, &client) => r,
                _ = &mut stop_rx => {
                    debug!("stream: stop requested");
                    return;
                }
            };

            match result {
                Ok(()) => {
                    debug!("stream: clean EOF, reconnecting immediately");
                    attempt = 0; // clean EOF (server-side restart) — reconnect fast
                }
                Err(e) => {
                    let backoff = (RETRY_BACKOFF_BASE_MS * (1u64 << attempt.min(5)))
                        .min(RETRY_BACKOFF_MAX_MS);
                    warn!("stream: error ({}) — retry in {}ms", e, backoff);
                    attempt += 1;
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                }
            }
        }
    })
}

/// Strips Shoutcast/Icecast (ICY) metadata blocks injected into the audio stream.
///
/// ICY streams inject metadata every `metaint` audio bytes.  After exactly
/// `metaint` audio bytes the server inserts one length-prefix byte (L) followed
/// by L×16 bytes of UTF-8 metadata.  We must discard those bytes so the MP3
/// decoder never sees them.
struct IcyStripper {
    metaint: usize,
    /// Audio bytes remaining before the next metadata block
    audio_remaining: usize,
    /// Metadata bytes remaining to skip
    meta_remaining: usize,
}

impl IcyStripper {
    fn new(metaint: usize) -> Self {
        Self {
            metaint,
            audio_remaining: metaint,
            meta_remaining: 0,
        }
    }

    fn process(&mut self, input: &[u8], output: &mut Vec<u8>) {
        for &byte in input {
            if self.meta_remaining > 0 {
                // Inside a metadata block — discard
                self.meta_remaining -= 1;
            } else if self.audio_remaining == 0 {
                // This byte is the length prefix: L × 16 = metadata block size
                self.meta_remaining = (byte as usize) * 16;
                self.audio_remaining = self.metaint;
            } else {
                // Normal audio byte — keep it
                output.push(byte);
                self.audio_remaining -= 1;
            }
        }
    }
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

    // Detect ICY metadata injection interval
    let metaint = response
        .headers()
        .get("icy-metaint")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok());

    if let Some(m) = metaint {
        debug!("stream: ICY metadata active, metaint={} bytes", m);
    }

    let mut stripper = metaint.map(IcyStripper::new);
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk: Bytes = chunk?;
        let mut buf = buffer.lock().unwrap();
        // Discard oldest bytes if buffer is full
        while buf.len() + chunk.len() > RING_BUFFER_CAPACITY {
            buf.pop_front();
        }
        if let Some(ref mut s) = stripper {
            let mut clean = Vec::with_capacity(chunk.len());
            s.process(&chunk, &mut clean);
            buf.extend(clean.iter());
        } else {
            buf.extend(chunk.iter());
        }
    }

    Ok(())
}
