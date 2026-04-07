use alsa::pcm::{Access, Format, HwParams, PCM};
use alsa::{Direction, ValueOr};

pub struct InfernoAlsaDevice {
    pcm: PCM,
    pub sample_rate: u32,
    pub channels: u32,
}

/// Build the ALSA device name for a given player slot.
/// Uses the named PCM `pcm.inferno_iradio_N` defined in ~/.asoundrc
/// (written by `alsa_setup::ensure_iradio_alsa` on startup).
pub fn device_name(slot: usize) -> String {
    format!("inferno_iradio_{}", slot)
}

impl InfernoAlsaDevice {
    pub fn open(device_str: &str, sample_rate: u32, buffer_frames: u32) -> anyhow::Result<Self> {
        let pcm = PCM::new(device_str, Direction::Playback, false)
            .map_err(|e| anyhow::anyhow!("ALSA open '{}' failed: {}", device_str, e))?;

        {
            let hwp = HwParams::any(&pcm)?;
            hwp.set_channels(2)?;
            hwp.set_rate(sample_rate, ValueOr::Nearest)?;
            hwp.set_format(Format::s32())?;
            hwp.set_access(Access::RWInterleaved)?;
            hwp.set_buffer_size(buffer_frames as i64)?;
            pcm.hw_params(&hwp)?;
        }

        pcm.prepare()?;

        Ok(Self {
            pcm,
            sample_rate,
            channels: 2,
        })
    }

    /// Write interleaved S32 samples (L, R, L, R, ...).
    /// Returns number of frames written, or error.
    pub fn write_frames(&self, samples: &[i32]) -> anyhow::Result<usize> {
        let io = self.pcm.io_i32()?;
        let frames = io
            .writei(samples)
            .map_err(|e| anyhow::anyhow!("ALSA write failed: {}", e))?;
        Ok(frames)
    }

    pub fn drain(&self) {
        let _ = self.pcm.drain();
    }
}

impl Drop for InfernoAlsaDevice {
    fn drop(&mut self) {
        let _ = self.pcm.drop();
    }
}
