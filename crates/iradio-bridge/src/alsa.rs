use alsa::pcm::{Access, Format, HwParams, PCM};
use alsa::{Direction, ValueOr};

pub struct InfernoAlsaDevice {
    pcm: PCM,
    pub _sample_rate: u32,
    pub channels: u32,
    pub buffer_frames: u32,
}

/// Build the ALSA device name for a given player slot.
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
            _sample_rate: sample_rate,
            channels: 2,
            buffer_frames,
        })
    }

    /// Write interleaved S32 samples (L, R, L, R, ...).
    /// On xrun (EPIPE/EIO), recovers and retries once.
    pub fn write_frames(&self, samples: &[i32]) -> anyhow::Result<usize> {
        let io = self.pcm.io_i32()?;
        match io.writei(samples) {
            Ok(n) => Ok(n),
            Err(e) => {
                // Try to recover from xrun (buffer underrun = EPIPE)
                if let Err(re) = self.pcm.recover(e.errno(), false) {
                    return Err(anyhow::anyhow!("ALSA write+recover failed: {} / {}", e, re));
                }
                // Retry once after recovery
                match io.writei(samples) {
                    Ok(n) => Ok(n),
                    Err(e2) => Err(anyhow::anyhow!("ALSA write failed after recovery: {}", e2)),
                }
            }
        }
    }

    /// Write one period of silence to keep the ALSA pipeline fed.
    pub fn write_silence(&self) {
        let silence = vec![0i32; (self.buffer_frames * self.channels) as usize];
        let _ = self.write_frames(&silence);
    }

    pub fn _drain(&self) {
        let _ = self.pcm.drain();
    }
}

impl Drop for InfernoAlsaDevice {
    fn drop(&mut self) {
        let _ = self.pcm.drop();
    }
}
