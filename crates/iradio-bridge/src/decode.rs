use rubato::{FftFixedIn, Resampler};
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
pub fn decode_to_pcm(
    source: Box<dyn MediaSource>,
    target_sample_rate: u32,
) -> anyhow::Result<(Vec<i32>, u32)> {
    let mss = MediaSourceStream::new(source, Default::default());
    let hint = Hint::new();
    let format_opts = FormatOptions::default();
    let meta_opts = MetadataOptions::default();
    let dec_opts = DecoderOptions::default();

    let probed =
        symphonia::default::get_probe().format(&hint, mss, &format_opts, &meta_opts)?;

    let mut format = probed.format;

    // Find the first audio track
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow::anyhow!("no audio track found"))?;

    let track_id = track.id;
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &dec_opts)?;

    let source_rate = track
        .codec_params
        .sample_rate
        .unwrap_or(target_sample_rate);

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
                        channels = audio_buf.spec().channels.count();
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
        resample_stereo(stereo_f32, source_rate, target_sample_rate)?
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

fn resample_stereo(
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
    let mut resampler = FftFixedIn::<f32>::new(
        from_rate as usize,
        to_rate as usize,
        chunk_size,
        2,
        2,
    )?;

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
