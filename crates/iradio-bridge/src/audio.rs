//! Per-slot gain staging and soft-clip limiting for iradio-bridge v2.0
//!
//! This module provides audio processing utilities including gain application,
//! soft-clip limiting, and level metering for stereo audio streams.

/// Apply gain in dB and soft-clip limiting to audio samples.
///
/// # Arguments
/// * `samples` - Mutable reference to audio samples (interleaved stereo: L, R, L, R, ...)
/// * `gain_db` - Gain in dB, range -6.0 to 6.0
///
/// # Processing
/// 1. Skips processing if |gain_db| < 0.01 (0dB fast-path)
/// 2. Converts gain_db to linear multiplier: 10^(gain_db/20)
/// 3. Applies gain to each sample
/// 4. Applies soft-clip limiter using tanh normalization
pub fn apply_gain_and_limit(samples: &mut [i32], gain_db: f32) {
    // Fast-path: skip processing for near-zero gain
    if gain_db.abs() < 0.01 {
        return;
    }

    // Convert dB to linear gain
    let gain_linear = 10_f32.powf(gain_db / 20.0);
    let threshold = 0.9_f32;
    let max_f32 = i32::MAX as f32;
    let inv_max = 1.0 / max_f32;

    // Single-pass processing: combine gain + tanh limiter (branchless-friendly)
    for s in samples.iter_mut() {
        // Apply gain
        let gained = *s as f32 * gain_linear;

        // Normalize to -1.0..1.0 and apply soft-clip limiting
        let normalized = gained * inv_max;
        let clipped = (normalized * threshold).tanh() / threshold.tanh();

        // Convert back to i32
        *s = (clipped.clamp(-1.0, 1.0) * max_f32) as i32;
    }
}

/// Convert gain in dB to linear multiplier.
///
/// # Formula
/// linear = 10^(db / 20)
pub fn _db_to_linear(db: f32) -> f32 {
    10_f32.powf(db / 20.0)
}

/// Convert linear gain to dB.
///
/// # Formula
/// db = 20 * log10(|linear|)
///
/// Returns silence floor (-96 dBFS) for values below 1e-10.
pub fn linear_to_db(linear: f32) -> f32 {
    20.0 * linear.abs().max(1e-10).log10()
}

/// Compute RMS level for stereo channels (dBFS).
///
/// # Arguments
/// * `samples` - Interleaved stereo samples: [L0, R0, L1, R1, ...]
///
/// # Returns
/// * `(left_db, right_db)` - RMS levels in dBFS
///
/// Returns `(-96.0, -96.0)` for empty input (silence floor).
pub fn rms_db(samples: &[i32]) -> (f32, f32) {
    if samples.is_empty() {
        return (-96.0, -96.0);
    }

    let max_f32 = i32::MAX as f32;
    let inv_max = 1.0 / max_f32;

    let mut left_sum_sq = 0.0_f32;
    let mut right_sum_sq = 0.0_f32;
    let mut left_count = 0_usize;
    let mut right_count = 0_usize;

    // Process interleaved stereo samples
    for (i, &sample) in samples.iter().enumerate() {
        let normalized = sample as f32 * inv_max;
        let sq = normalized * normalized;

        if i % 2 == 0 {
            // Even index: left channel
            left_sum_sq += sq;
            left_count += 1;
        } else {
            // Odd index: right channel
            right_sum_sq += sq;
            right_count += 1;
        }
    }

    // Compute RMS and convert to dBFS
    let left_rms = (left_sum_sq / left_count as f32).sqrt();
    let right_rms = (right_sum_sq / right_count as f32).sqrt();

    (linear_to_db(left_rms), linear_to_db(right_rms))
}
