use alsa::pcm::{Access, Format, HwParams, PCM};
use alsa::{Direction, ValueOr};

pub struct InfernoAlsaDevice {
    pcm: PCM,
    pub sample_rate: u32,
    pub channels: u32,
}

/// Build the ALSA device name for a given player slot.
/// Matches the asoundrc `pcm.inferno` template parameters.
pub fn device_name(prefix: &str, slot: usize, alt_port_base: u16) -> String {
    // ALSA key=value string for the inferno PCM plugin
    // PROCESS_ID base: 10 (slot 1 = 10, slot 2 = 11, etc.)
    let process_id = 10 + slot - 1;
    let alt_port = alt_port_base + ((slot - 1) as u16) * 20;
    format!(
        "inferno:NAME={prefix}-{slot},TX_CHANNELS=2,RX_CHANNELS=0,\
         PROCESS_ID={process_id},ALT_PORT={alt_port}"
    )
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
