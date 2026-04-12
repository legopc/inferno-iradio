use bytes::Bytes;
use futures_util::StreamExt;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, info, warn};

const RING_BUFFER_CAPACITY: usize = 512 * 1024; // 512 KB — larger buffer for smoother decode
const RETRY_BACKOFF_BASE_MS: u64 = 500;
const RETRY_BACKOFF_MAX_MS: u64 = 10_000;
/// Minimum bytes received before we consider the effective_url "trustworthy".
/// If a stream errors before this threshold, we reset to the original URL so
/// a pre-roll ad can't permanently hijack the reconnect target.
const EFFECTIVE_URL_TRUST_BYTES: usize = 64 * 1024; // 64 KB

/// Shared ring buffer between fetcher and decoder
pub type SharedBuffer = Arc<Mutex<VecDeque<u8>>>;

/// Start a streaming HTTP fetch task. Writes bytes into `buffer`.
/// Retries indefinitely on EOF/error with exponential backoff (capped at 10 s).
/// Stops when `stop_rx` is signalled.
/// `title_tx` is an optional watch channel to push ICY metadata titles to.
pub fn start_stream_fetch(
    url: String,
    buffer: SharedBuffer,
    client: reqwest::Client,
    mut stop_rx: tokio::sync::oneshot::Receiver<()>,
    title_tx: Option<tokio::sync::watch::Sender<Option<String>>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut attempt = 0u32;
        // original_url never changes — used to reset if we get stuck on a pre-roll URL
        let original_url = url.clone();
        // After the first successful connect we use the final (post-redirect) URL
        // so reconnects skip any pre-roll ad served by redirect landing pages.
        // Only trusted once EFFECTIVE_URL_TRUST_BYTES have been received.
        let mut effective_url = url.clone();

        loop {
            debug!(
                "stream: connecting to {} (attempt {})",
                effective_url,
                attempt + 1
            );

            let result = tokio::select! {
                r = fetch_stream(&effective_url, &buffer, &client, title_tx.clone()) => r,
                _ = &mut stop_rx => {
                    debug!("stream: stop requested");
                    return;
                }
            };

            match result {
                Ok((final_url, bytes_received)) => {
                    // Only trust the redirect URL if we received enough data to be sure
                    // it's the real stream and not a pre-roll ad server
                    if let Some(u) = final_url {
                        if u != effective_url && bytes_received >= EFFECTIVE_URL_TRUST_BYTES {
                            info!("stream: using direct URL for reconnects: {}", u);
                            effective_url = u;
                        }
                    }
                    debug!(
                        "stream: clean EOF after {} bytes, reconnecting",
                        bytes_received
                    );
                    // Don't clear the buffer on clean EOF — let symphonia continue smoothly
                    attempt = 0;
                }
                Err((e, final_url, bytes_received)) => {
                    if bytes_received >= EFFECTIVE_URL_TRUST_BYTES {
                        // We got enough data — the redirect target is the real stream
                        if let Some(u) = final_url {
                            if u != effective_url {
                                effective_url = u;
                            }
                        }
                    } else {
                        // Too little data — likely a pre-roll or bad redirect.
                        // Reset to original URL so we don't get stuck.
                        if effective_url != original_url {
                            warn!(
                                "stream: only {} bytes before error, resetting to original URL",
                                bytes_received
                            );
                            effective_url = original_url.clone();
                        }
                    }
                    let backoff = (RETRY_BACKOFF_BASE_MS * (1u64 << attempt.min(5)))
                        .min(RETRY_BACKOFF_MAX_MS);
                    warn!("stream: error ({}) — retry in {}ms", e, backoff);
                    // Clear stale/corrupt bytes before reconnecting
                    buffer.lock().unwrap().clear();
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
    /// Accumulates metadata bytes for parsing
    meta_buf: Vec<u8>,
}

impl IcyStripper {
    fn new(metaint: usize) -> Self {
        Self {
            metaint,
            audio_remaining: metaint,
            meta_remaining: 0,
            meta_buf: Vec::new(),
        }
    }

    fn process(&mut self, input: &[u8], output: &mut Vec<u8>) {
        for &byte in input {
            if self.meta_remaining > 0 {
                // Inside a metadata block — collect bytes
                self.meta_buf.push(byte);
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

    fn take_title(&mut self) -> Option<String> {
        if self.meta_buf.is_empty() {
            return None;
        }
        let raw = String::from_utf8_lossy(&self.meta_buf).to_string();
        self.meta_buf.clear();
        // ICY metadata format: "StreamTitle='Artist - Track';StreamUrl='...';"
        // Extract StreamTitle value
        if let Some(start) = raw.find("StreamTitle='") {
            let rest = &raw[start + 13..];
            if let Some(end) = rest.find("';") {
                let title = rest[..end].trim().to_string();
                if !title.is_empty() {
                    return Some(title);
                }
            }
        }
        None
    }
}

async fn fetch_stream(
    url: &str,
    buffer: &SharedBuffer,
    client: &reqwest::Client,
    title_tx: Option<tokio::sync::watch::Sender<Option<String>>>,
) -> Result<(Option<String>, usize), (anyhow::Error, Option<String>, usize)> {
    let response = client
        .get(url)
        // Tell servers not to compress — we need raw audio bytes
        .header("Accept-Encoding", "identity")
        .header("Icy-MetaData", "1")
        .send()
        .await
        .map_err(|e| (e.into(), None, 0usize))?
        .error_for_status()
        .map_err(|e| (e.into(), None, 0usize))?;

    // Capture the final URL after redirects so reconnects skip pre-roll ads
    let final_url = Some(response.url().to_string());

    // Log connection details for diagnostics
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("?")
        .to_string();
    let content_encoding = response
        .headers()
        .get("content-encoding")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("none")
        .to_string();
    let icy_name = response
        .headers()
        .get("icy-name")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let server = response
        .headers()
        .get("server")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("?")
        .to_string();
    info!(
        "stream: connected {} | type={} enc={} icy-name={:?} server={} url={}",
        status,
        content_type,
        content_encoding,
        icy_name,
        server,
        final_url.as_deref().unwrap_or(url)
    );

    // Detect ICY metadata injection interval (filter out metaint=0 bug)
    let metaint = response
        .headers()
        .get("icy-metaint")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok().filter(|&n| n > 0));

    if let Some(m) = metaint {
        debug!("stream: ICY metadata active, metaint={} bytes", m);
    }

    let mut stripper = metaint.map(IcyStripper::new);
    let mut stream = response.bytes_stream();
    let mut bytes_received: usize = 0;

    while let Some(chunk) = stream.next().await {
        let chunk: Bytes = chunk.map_err(|e| (e.into(), final_url.clone(), bytes_received))?;
        bytes_received += chunk.len();
        let mut buf = buffer.lock().unwrap();
        // Discard oldest bytes if buffer is full
        while buf.len() + chunk.len() > RING_BUFFER_CAPACITY {
            buf.pop_front();
        }
        if let Some(ref mut s) = stripper {
            let mut clean = Vec::with_capacity(chunk.len());
            s.process(&chunk, &mut clean);
            // Push any new ICY title
            if let Some(title) = s.take_title() {
                if let Some(ref tx) = title_tx {
                    let _ = tx.send(Some(title));
                }
            }
            buf.extend(clean.iter());
        } else {
            buf.extend(chunk.iter());
        }
    }

    Ok((final_url, bytes_received))
}
