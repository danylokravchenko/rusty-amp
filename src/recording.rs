use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

pub struct RecordingState {
    pub active: Arc<AtomicBool>,
    pub buffer: Arc<Mutex<Vec<f32>>>,
    pub sample_rate: Arc<AtomicU32>,
}

impl RecordingState {
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: Arc::new(AtomicU32::new(44100)),
        }
    }

    pub fn start(&self) {
        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }
        self.active.store(true, Relaxed);
    }

    pub fn stop_and_save(&self) -> Result<PathBuf> {
        self.active.store(false, Relaxed);
        let samples = self
            .buffer
            .lock()
            .map_err(|_| anyhow!("recording buffer lock poisoned"))
            .map(|mut g| std::mem::take(&mut *g))?;
        let sr = self.sample_rate.load(Relaxed);
        save_wav(&samples, sr)
    }
}

fn save_wav(samples: &[f32], sample_rate: u32) -> Result<PathBuf> {
    let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = base.join(format!("rusty-amp-{secs}.wav"));

    // Recording buffer holds interleaved stereo (L, R) frames.
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(&path, spec)?;
    for &s in samples {
        writer.write_sample(s)?;
    }
    writer.finalize()?;
    Ok(path)
}
