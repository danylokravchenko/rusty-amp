//! Practice metronome: a steady click mixed into the monitor output only.
//!
//! The click is generated on the audio thread by a [`MetronomeVoice`] and added
//! to the signal that feeds the speakers/headphones — but *after* the recording
//! tap, so an engaged metronome is never captured in the rendered WAV. Tempo and
//! on/off state live in the shared [`Metronome`], which the UI writes and the
//! audio thread reads.

use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

/// Tempo bounds (beats per minute) exposed by the UI slider.
pub const MIN_BPM: u32 = 40;
pub const MAX_BPM: u32 = 240;
/// Tempo the metronome starts at.
pub const DEFAULT_BPM: u32 = 120;

/// Click tone frequency (Hz) and its exponential decay rate (1/s). A short, high
/// blip cuts through a distorted guitar without being harsh.
const CLICK_FREQ: f32 = 1000.0;
const CLICK_DECAY: f32 = 55.0;
/// Click duration in seconds — long enough to hear, short enough to stay tight.
const CLICK_SECS: f32 = 0.04;
/// Peak click amplitude in the output mix.
const CLICK_GAIN: f32 = 0.5;

/// Live metronome state shared between the UI and audio threads.
///
/// The UI writes both fields; the audio thread only reads them. `bpm` is clamped
/// to [`MIN_BPM`]..=[`MAX_BPM`] by [`Metronome::set_bpm`].
pub struct Metronome {
    /// UI → audio: when true, the click is mixed into the monitor output.
    pub active: AtomicBool,
    /// UI → audio: tempo in beats per minute.
    pub bpm: AtomicU32,
}

impl Default for Metronome {
    fn default() -> Self {
        Self::new()
    }
}

impl Metronome {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            bpm: AtomicU32::new(DEFAULT_BPM),
        }
    }

    /// Current tempo (always within the valid range).
    pub fn get_bpm(&self) -> u32 {
        self.bpm.load(Relaxed).clamp(MIN_BPM, MAX_BPM)
    }

    /// Set the tempo, clamped to the valid range.
    pub fn set_bpm(&self, bpm: u32) {
        self.bpm.store(bpm.clamp(MIN_BPM, MAX_BPM), Relaxed);
    }

    /// Nudge the tempo by `delta` BPM (saturating at the range bounds).
    pub fn nudge_bpm(&self, delta: i32) {
        let next = (self.get_bpm() as i32 + delta).clamp(MIN_BPM as i32, MAX_BPM as i32);
        self.bpm.store(next as u32, Relaxed);
    }

    /// Flip the metronome on/off; returns the new state.
    pub fn toggle(&self) -> bool {
        let now = !self.active.load(Relaxed);
        self.active.store(now, Relaxed);
        now
    }
}

/// Audio-thread click generator. All scratch is fixed-size, so `next_sample`
/// never allocates. One instance drives one mono click stream.
pub struct MetronomeVoice {
    sr: f32,
    /// Length of one click in samples.
    click_len: usize,
    /// Samples elapsed since the last beat onset; a beat fires when it reaches
    /// the current period.
    since_beat: f32,
    /// Position within the currently sounding click, or `click_len` when idle.
    click_pos: usize,
}

impl MetronomeVoice {
    pub fn new(sr: f32) -> Self {
        let click_len = (CLICK_SECS * sr).round() as usize;
        Self {
            sr,
            click_len,
            // Infinity so the first active frame triggers a click immediately;
            // the non-finite carry branch below then zeroes it cleanly.
            since_beat: f32::INFINITY,
            click_pos: click_len,
        }
    }

    /// Advance one frame and return the click sample to add to the output.
    ///
    /// `active` gates the whole voice; `bpm` sets the tempo. When inactive the
    /// voice resets and returns silence, so re-engaging clicks on the first beat.
    pub fn next_sample(&mut self, active: bool, bpm: u32) -> f32 {
        if !active {
            self.since_beat = f32::INFINITY;
            self.click_pos = self.click_len;
            return 0.0;
        }

        let period = self.sr * 60.0 / bpm.max(1) as f32;
        if self.since_beat >= period {
            // Carry the remainder so tempo stays accurate over time.
            self.since_beat = if self.since_beat.is_finite() {
                self.since_beat - period
            } else {
                0.0
            };
            self.click_pos = 0;
        }
        self.since_beat += 1.0;

        if self.click_pos < self.click_len {
            let t = self.click_pos as f32 / self.sr;
            let env = (-t * CLICK_DECAY).exp();
            let s = (2.0 * PI * CLICK_FREQ * t).sin() * env * CLICK_GAIN;
            self.click_pos += 1;
            s
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bpm_is_clamped_to_range() {
        let m = Metronome::new();
        m.set_bpm(10_000);
        assert_eq!(m.get_bpm(), MAX_BPM);
        m.set_bpm(1);
        assert_eq!(m.get_bpm(), MIN_BPM);
    }

    #[test]
    fn nudge_saturates_at_bounds() {
        let m = Metronome::new();
        m.set_bpm(MIN_BPM);
        m.nudge_bpm(-100);
        assert_eq!(m.get_bpm(), MIN_BPM);
        m.set_bpm(MAX_BPM);
        m.nudge_bpm(100);
        assert_eq!(m.get_bpm(), MAX_BPM);
    }

    #[test]
    fn inactive_voice_is_silent() {
        let mut v = MetronomeVoice::new(48_000.0);
        for _ in 0..10_000 {
            assert_eq!(v.next_sample(false, 120), 0.0);
        }
    }

    /// At a known tempo the number of clicks over one second matches the BPM,
    /// and each beat lands within a sample of where it should.
    #[test]
    fn clicks_land_on_the_beat() {
        let sr = 48_000.0;
        let bpm = 120u32; // two beats per second
        let mut v = MetronomeVoice::new(sr);

        let mut onsets = Vec::new();
        for n in 0..sr as usize {
            v.next_sample(true, bpm);
            // The onset frame is the one that reset the click and advanced its
            // position to 1 (the click's first sample is silent, so watch state).
            if v.click_pos == 1 {
                onsets.push(n);
            }
        }
        // 120 BPM over one second → the onset at t≈0 plus two full periods.
        assert!(
            (onsets.len() as i32 - 2).abs() <= 1,
            "expected ~2 beats/sec, got {}",
            onsets.len()
        );
        let period = sr * 60.0 / bpm as f32;
        for pair in onsets.windows(2) {
            let gap = (pair[1] - pair[0]) as f32;
            assert!((gap - period).abs() <= 1.0, "beat spacing off: {gap}");
        }
    }
}
