use rubato::{FftFixedIn, Resampler};
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tracing::{debug, warn};

/// Decode audio from `source` into interleaved i32 PCM at `target_sample_rate`.
/// `source` must implement `MediaSource` (i.e. `Read + Seek + Send + Sync`).
/// Returns the decoded samples and the detected source sample rate.
pub fn _decode_to_pcm(
    source: Box<dyn MediaSource>,
    target_sample_rate: u32,
) -> anyhow::Result<(Vec<i32>, u32)> {
    let mss = MediaSourceStream::new(source, Default::default());
    let hint = Hint::new();
    let format_opts = FormatOptions::default();
    let meta_opts = MetadataOptions::default();
    let dec_opts = DecoderOptions::default();

    let probed = symphonia::default::get_probe().format(&hint, mss, &format_opts, &meta_opts)?;

    let mut format = probed.format;

    // Find the first audio track
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow::anyhow!("no audio track found"))?;

    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &dec_opts)?;

    let source_rate = track.codec_params.sample_rate.unwrap_or(target_sample_rate);

    let mut samples_f32: Vec<f32> = Vec::new();
    let mut channels = 2usize;

    loop {
        match format.next_packet() {
            Ok(packet) => {
                if packet.track_id() != track_id {
                    continue;
                }
                match decoder.decode(&packet) {
                    Ok(audio_buf) => {
                        channels = audio_buf.spec().channels.count().max(1);
                        convert_to_f32(&audio_buf, &mut samples_f32, channels);
                    }
                    Err(symphonia::core::errors::Error::DecodeError(e)) => {
                        warn!("decode error: {}", e);
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            Err(symphonia::core::errors::Error::IoError(_)) => break,
            Err(symphonia::core::errors::Error::ResetRequired) => {
                decoder.reset();
            }
            Err(e) => return Err(e.into()),
        }
    }

    // Ensure stereo
    let stereo_f32 = if channels == 1 {
        // Mono → stereo duplication
        samples_f32
            .iter()
            .flat_map(|&s| [s, s])
            .collect::<Vec<f32>>()
    } else {
        samples_f32
    };

    // Resample if needed
    let final_f32 = if source_rate != target_sample_rate {
        debug!(
            "resampling {} → {} Hz ({} frames)",
            source_rate,
            target_sample_rate,
            stereo_f32.len() / 2
        );
        _resample_stereo(stereo_f32, source_rate, target_sample_rate)?
    } else {
        stereo_f32
    };

    // Convert f32 to i32 (S32_LE)
    let i32_samples: Vec<i32> = final_f32
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * i32::MAX as f32) as i32)
        .collect();

    Ok((i32_samples, source_rate))
}

fn convert_to_f32(buf: &AudioBufferRef, out: &mut Vec<f32>, channels: usize) {
    match buf {
        AudioBufferRef::F32(b) => {
            for frame in 0..b.frames() {
                for ch in 0..channels.min(2) {
                    out.push(b.chan(ch)[frame]);
                }
            }
        }
        AudioBufferRef::S16(b) => {
            for frame in 0..b.frames() {
                for ch in 0..channels.min(2) {
                    out.push(b.chan(ch)[frame] as f32 / i16::MAX as f32);
                }
            }
        }
        AudioBufferRef::S32(b) => {
            for frame in 0..b.frames() {
                for ch in 0..channels.min(2) {
                    out.push(b.chan(ch)[frame] as f32 / i32::MAX as f32);
                }
            }
        }
        _ => {
            warn!("unsupported sample format, skipping buffer");
        }
    }
}

fn _resample_stereo(
    interleaved: Vec<f32>,
    from_rate: u32,
    to_rate: u32,
) -> anyhow::Result<Vec<f32>> {
    // Deinterleave
    let frames = interleaved.len() / 2;
    let mut left: Vec<f32> = Vec::with_capacity(frames);
    let mut right: Vec<f32> = Vec::with_capacity(frames);
    for chunk in interleaved.chunks_exact(2) {
        left.push(chunk[0]);
        right.push(chunk[1]);
    }

    let chunk_size = 1024usize;
    let mut resampler =
        FftFixedIn::<f32>::new(from_rate as usize, to_rate as usize, chunk_size, 2, 2)?;

    let mut out_left: Vec<f32> = Vec::new();
    let mut out_right: Vec<f32> = Vec::new();

    let mut offset = 0;
    while offset + chunk_size <= left.len() {
        let waves = vec![
            left[offset..offset + chunk_size].to_vec(),
            right[offset..offset + chunk_size].to_vec(),
        ];
        let resampled = resampler.process(&waves, None)?;
        out_left.extend_from_slice(&resampled[0]);
        out_right.extend_from_slice(&resampled[1]);
        offset += chunk_size;
    }

    // Re-interleave
    let result: Vec<f32> = out_left
        .into_iter()
        .zip(out_right)
        .flat_map(|(l, r)| [l, r])
        .collect();

    Ok(result)
}

// ── Streaming decode ──────────────────────────────────────────────────────────

use crate::stream::SharedBuffer;

/// A `MediaSource` that reads from the shared HTTP byte buffer.
/// Blocks (spinning with 5 ms sleep) when the buffer is empty.
/// Returns 0 bytes (EOF) when `stop` is set.
struct StreamingMediaSource {
    buffer: SharedBuffer,
    stop: Arc<AtomicBool>,
    pos: u64,
}

impl Read for StreamingMediaSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.stop.load(Ordering::Relaxed) {
                return Ok(0); // Signal EOF to symphonia
            }
            let mut inner = self.buffer.lock().unwrap();
            if inner.is_empty() {
                drop(inner);
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            let take = buf.len().min(inner.len());
            for (i, b) in inner.drain(..take).enumerate() {
                buf[i] = b;
            }
            self.pos += take as u64;
            return Ok(take);
        }
    }
}

impl Seek for StreamingMediaSource {
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "non-seekable"))
    }
}

impl MediaSource for StreamingMediaSource {
    fn is_seekable(&self) -> bool {
        false
    }
    fn byte_len(&self) -> Option<u64> {
        None
    }
}

/// Handle returned by `start_streaming_decode`. Drop or call `stop()` to halt
/// the decode thread.
pub struct StreamingDecodeHandle {
    pub stop: Arc<AtomicBool>,
    _thread: Option<std::thread::JoinHandle<()>>,
}

impl StreamingDecodeHandle {
    /// Signal the decode thread to stop and wait for it.
    pub fn _stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self._thread.take() {
            let _ = t.join();
        }
    }
}

/// Start a background blocking thread that continuously decodes the HTTP stream
/// and sends period-sized interleaved i32 PCM chunks via a tokio mpsc channel.
///
/// The channel has capacity 4 (≈340 ms of audio at 48 kHz / 4096 frames) to
/// provide backpressure without large latency.
pub fn start_streaming_decode(
    buffer: SharedBuffer,
    target_sample_rate: u32,
    period_frames: usize,
) -> (StreamingDecodeHandle, tokio::sync::mpsc::Receiver<Vec<i32>>) {
    let stop = Arc::new(AtomicBool::new(false));
    let (pcm_tx, pcm_rx) = tokio::sync::mpsc::channel::<Vec<i32>>(4);

    let stop2 = stop.clone();
    let thread = std::thread::spawn(move || {
        decode_stream_thread(buffer, stop2, target_sample_rate, period_frames, pcm_tx);
    });

    (
        StreamingDecodeHandle {
            stop,
            _thread: Some(thread),
        },
        pcm_rx,
    )
}

fn decode_stream_thread(
    buffer: SharedBuffer,
    stop: Arc<AtomicBool>,
    target_sample_rate: u32,
    period_frames: usize,
    pcm_tx: tokio::sync::mpsc::Sender<Vec<i32>>,
) {
    let source = StreamingMediaSource {
        buffer,
        stop: stop.clone(),
        pos: 0,
    };
    let mss = MediaSourceStream::new(Box::new(source), Default::default());

    let probed = match symphonia::default::get_probe().format(
        &Hint::new(),
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    ) {
        Ok(p) => p,
        Err(e) => {
            warn!("streaming decode: probe failed: {}", e);
            return;
        }
    };

    let mut format = probed.format;

    let track = match format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
    {
        Some(t) => t,
        None => {
            warn!("streaming decode: no audio track found");
            return;
        }
    };

    let track_id = track.id;
    let source_rate = track.codec_params.sample_rate.unwrap_or(target_sample_rate);
    let need_resample = source_rate != target_sample_rate;
    let resample_chunk = 1024usize;

    let mut decoder = match symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
    {
        Ok(d) => d,
        Err(e) => {
            warn!("streaming decode: codec init failed: {}", e);
            return;
        }
    };

    let mut resampler: Option<FftFixedIn<f32>> = if need_resample {
        match FftFixedIn::new(
            source_rate as usize,
            target_sample_rate as usize,
            resample_chunk,
            2,
            2,
        ) {
            Ok(r) => {
                debug!(
                    "streaming decode: resampler {} → {} Hz",
                    source_rate, target_sample_rate
                );
                Some(r)
            }
            Err(e) => {
                warn!("streaming decode: resampler init failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Accumulate stereo f32 samples until we have a full period.
    let mut acc: Vec<f32> = Vec::with_capacity(period_frames * 4);
    let stride = period_frames * 2; // stereo samples per period

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        match format.next_packet() {
            Ok(packet) => {
                if packet.track_id() != track_id {
                    continue;
                }
                match decoder.decode(&packet) {
                    Ok(audio_buf) => {
                        let ch = audio_buf.spec().channels.count().max(1);
                        convert_to_f32(&audio_buf, &mut acc, ch);

                        // Flush complete periods
                        while acc.len() >= stride {
                            let chunk_f32: Vec<f32> = acc.drain(..stride).collect();

                            let final_f32 = if let Some(ref mut rs) = resampler {
                                // Deinterleave
                                let frames = chunk_f32.len() / 2;
                                let mut left: Vec<f32> = Vec::with_capacity(frames);
                                let mut right: Vec<f32> = Vec::with_capacity(frames);
                                for s in chunk_f32.chunks_exact(2) {
                                    left.push(s[0]);
                                    right.push(s[1]);
                                }
                                let mut out_l = Vec::new();
                                let mut out_r = Vec::new();
                                let mut off = 0;
                                while off + resample_chunk <= left.len() {
                                    let waves = vec![
                                        left[off..off + resample_chunk].to_vec(),
                                        right[off..off + resample_chunk].to_vec(),
                                    ];
                                    if let Ok(out) = rs.process(&waves, None) {
                                        out_l.extend_from_slice(&out[0]);
                                        out_r.extend_from_slice(&out[1]);
                                    }
                                    off += resample_chunk;
                                }
                                out_l
                                    .into_iter()
                                    .zip(out_r)
                                    .flat_map(|(l, r)| [l, r])
                                    .collect::<Vec<f32>>()
                            } else {
                                chunk_f32
                            };

                            let i32_samples: Vec<i32> = final_f32
                                .iter()
                                .map(|&s| (s.clamp(-1.0, 1.0) * i32::MAX as f32) as i32)
                                .collect();

                            if pcm_tx.blocking_send(i32_samples).is_err() {
                                return; // Receiver dropped (player stopped)
                            }
                        }
                    }
                    Err(symphonia::core::errors::Error::DecodeError(e)) => {
                        warn!("streaming decode: frame error (skipping): {}", e);
                    }
                    Err(e) => {
                        warn!("streaming decode: fatal decode error: {}", e);
                        break;
                    }
                }
            }
            Err(symphonia::core::errors::Error::IoError(_)) => {
                // EOF or stop signal — check stop flag
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                // Otherwise the source is just waiting for more bytes (shouldn't reach here
                // since StreamingMediaSource blocks on read rather than returning IoError)
            }
            Err(symphonia::core::errors::Error::ResetRequired) => {
                decoder.reset();
            }
            Err(e) => {
                warn!("streaming decode: format error: {}", e);
                break;
            }
        }
    }
}
