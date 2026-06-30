//! Guitar tuner: pitch detection + a coarse note spectrum.
//!
//! When the tuner is engaged the audio thread bypasses the whole rig (pedals,
//! amp, cab, effects) and feeds the dry guitar straight through, so the player
//! hears a clean signal while tuning. The same dry block is handed to a
//! [`TunerDetector`], which estimates the played pitch with the McLeod normalised
//! square-difference function (NSDF) and renders a log-spaced magnitude spectrum
//! with a bank of Goertzel filters. Both results are published to the shared
//! [`Tuner`] state for the UI to read.

use atomic_float::AtomicF32;
use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

/// Number of log-spaced magnitude bars in the displayed spectrum.
pub const SPECTRUM_BINS: usize = 64;

/// Spectrum display range (Hz) — low E (drop tunings) up past the top of the neck.
const SPEC_FMIN: f32 = 60.0;
const SPEC_FMAX: f32 = 1200.0;

/// Pitch-detection search range (Hz). Wide enough for drop tunings and lead notes.
const DET_FMIN: f32 = 55.0;
const DET_FMAX: f32 = 1400.0;

/// Analysis window and hop (samples). The window is ~43 ms at 48 kHz — long
/// enough for a couple of periods of the lowest notes, short enough to track.
const WINDOW: usize = 2048;
const HOP: usize = 1024;

/// Below this input RMS the window is treated as silence (no note played).
const NOISE_RMS: f32 = 0.004;
/// Minimum NSDF peak height for a pitch estimate to be trusted.
const CLARITY_MIN: f32 = 0.6;

/// Centre frequency (Hz) of spectrum bar `i`, log-spaced across the display range.
pub fn spectrum_bin_freq(i: usize) -> f32 {
    let t = i as f32 / (SPECTRUM_BINS - 1) as f32;
    SPEC_FMIN * (SPEC_FMAX / SPEC_FMIN).powf(t)
}

/// Live tuner readout shared between the audio and UI threads.
///
/// `active` is the one field the UI writes: setting it engages the clean-signal
/// bypass in the audio callback. Everything else flows audio → UI.
pub struct Tuner {
    /// UI → audio: when true, the rig is bypassed and the detector runs.
    pub active: AtomicBool,
    /// Detected fundamental in Hz, or 0.0 when no confident pitch is present.
    pub freq: AtomicF32,
    /// Confidence of the pitch estimate (NSDF peak height), 0.0–1.0.
    pub clarity: AtomicF32,
    /// Normalised (0.0–1.0) magnitude per log-spaced spectrum bar.
    pub spectrum: [AtomicF32; SPECTRUM_BINS],
}

impl Default for Tuner {
    fn default() -> Self {
        Self::new()
    }
}

impl Tuner {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            freq: AtomicF32::new(0.0),
            clarity: AtomicF32::new(0.0),
            spectrum: std::array::from_fn(|_| AtomicF32::new(0.0)),
        }
    }
}

/// The nearest equal-tempered note to a frequency, and how far off it is.
pub struct NoteReading {
    /// Note letter (with sharp), e.g. "E" or "A#".
    pub name: &'static str,
    /// Scientific-pitch octave number (A4 = 440 Hz).
    pub octave: i32,
    /// Signed offset from the nearest note in cents (−50.0 … +50.0).
    pub cents: f32,
}

/// Map a frequency to its nearest equal-tempered note (A4 = 440 Hz).
pub fn note_of(freq: f32) -> NoteReading {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let midi = 69.0 + 12.0 * (freq / 440.0).log2();
    let nearest = midi.round();
    let cents = (midi - nearest) * 100.0;
    let idx = (nearest as i32).rem_euclid(12) as usize;
    let octave = (nearest as i32).div_euclid(12) - 1;
    NoteReading {
        name: NAMES[idx],
        octave,
        cents,
    }
}

/// Accumulates dry input samples and, once per hop, publishes a fresh pitch and
/// spectrum to a shared [`Tuner`]. All scratch is pre-allocated, so `process`
/// never allocates on the audio thread.
pub struct TunerDetector {
    sr: f32,
    /// Rolling input accumulator; analysed and slid forward by `HOP` each time it
    /// reaches `WINDOW` samples.
    buf: Vec<f32>,
    /// DC-removed copy of the current analysis window.
    work: Vec<f32>,
    /// NSDF scratch, indexed by lag.
    nsdf: Vec<f32>,
    /// Centre frequency of each spectrum bar.
    bins: [f32; SPECTRUM_BINS],
    min_lag: usize,
    max_lag: usize,
}

impl TunerDetector {
    pub fn new(sr: f32) -> Self {
        let min_lag = (sr / DET_FMAX).floor().max(2.0) as usize;
        let max_lag = ((sr / DET_FMIN).ceil() as usize).min(WINDOW - 1);
        Self {
            sr,
            buf: Vec::with_capacity(WINDOW * 2 + crate::audio::MAX_BLOCK),
            work: vec![0.0; WINDOW],
            nsdf: vec![0.0; max_lag + 1],
            bins: std::array::from_fn(spectrum_bin_freq),
            min_lag,
            max_lag,
        }
    }

    /// Feed one dry input block and publish a new reading whenever a full window
    /// has accumulated.
    pub fn process(&mut self, input: &[f32], out: &Tuner) {
        self.buf.extend_from_slice(input);
        while self.buf.len() >= WINDOW {
            self.analyze(out);
            let drop = HOP.min(self.buf.len());
            self.buf.drain(0..drop);
        }
    }

    fn analyze(&mut self, out: &Tuner) {
        // DC-remove the window into `work` and measure its level.
        let mean = self.buf[..WINDOW].iter().sum::<f32>() / WINDOW as f32;
        let mut sumsq = 0.0f32;
        for (w, &s) in self.work.iter_mut().zip(&self.buf[..WINDOW]) {
            let v = s - mean;
            *w = v;
            sumsq += v * v;
        }
        let rms = (sumsq / WINDOW as f32).sqrt();

        if rms < NOISE_RMS {
            // No note: clear the pitch and let the bars fall away smoothly.
            out.freq.store(0.0, Relaxed);
            out.clarity.store(0.0, Relaxed);
            for b in &out.spectrum {
                b.store(b.load(Relaxed) * 0.6, Relaxed);
            }
            return;
        }

        let (freq, clarity) = self.detect_pitch();
        out.freq.store(freq, Relaxed);
        out.clarity.store(clarity, Relaxed);

        // Log-spaced magnitude spectrum, normalised to its own peak so the bars
        // always fill the display, then time-smoothed for a steadier readout.
        let mut mags = [0.0f32; SPECTRUM_BINS];
        let mut peak = 1e-9f32;
        for (m, &f) in mags.iter_mut().zip(&self.bins) {
            *m = goertzel(&self.work, f, self.sr);
            peak = peak.max(*m);
        }
        for (cell, m) in out.spectrum.iter().zip(mags) {
            let norm = (m / peak).clamp(0.0, 1.0);
            let prev = cell.load(Relaxed);
            cell.store(prev * 0.5 + norm * 0.5, Relaxed);
        }
    }

    /// McLeod pitch estimate over `self.work`: returns (Hz, clarity). A frequency
    /// of 0.0 means no confident pitch was found.
    fn detect_pitch(&mut self) -> (f32, f32) {
        let x = &self.work;
        let n = x.len();
        let (lo, hi) = (self.min_lag, self.max_lag.min(n - 1));

        for tau in lo..=hi {
            let mut ac = 0.0f32;
            let mut m = 0.0f32;
            for i in 0..(n - tau) {
                ac += x[i] * x[i + tau];
                m += x[i] * x[i] + x[i + tau] * x[i + tau];
            }
            self.nsdf[tau] = if m > 0.0 { 2.0 * ac / m } else { 0.0 };
        }

        // MPM peak picking: the key maximum of each positive lobe of the NSDF.
        let nsdf = &self.nsdf;
        let mut tau = lo;
        // Skip the leading descent so we don't latch onto the zero-lag lobe.
        while tau <= hi && nsdf[tau] > 0.0 {
            tau += 1;
        }
        let mut best_peak = 0usize;
        let mut highest = 0.0f32;
        let mut first_peak = 0usize;
        let mut first_above = false;
        while tau <= hi {
            while tau <= hi && nsdf[tau] <= 0.0 {
                tau += 1;
            }
            let mut max_pos = tau.min(hi);
            while tau <= hi && nsdf[tau] > 0.0 {
                if nsdf[tau] > nsdf[max_pos] {
                    max_pos = tau;
                }
                tau += 1;
            }
            if max_pos <= hi && nsdf[max_pos] > 0.0 {
                if nsdf[max_pos] > highest {
                    highest = nsdf[max_pos];
                    best_peak = max_pos;
                }
                // Remember the first sufficiently strong peak; for an octave-rich
                // guitar tone the true fundamental is the earliest tall lobe.
                if !first_above && nsdf[max_pos] >= CLARITY_MIN {
                    first_peak = max_pos;
                    first_above = true;
                }
            }
        }

        if highest < CLARITY_MIN {
            return (0.0, highest.max(0.0));
        }
        // Prefer the first strong lobe if it's nearly as tall as the global best.
        let peak = if first_above && nsdf[first_peak] >= 0.9 * highest {
            first_peak
        } else {
            best_peak
        };

        let tau_est = parabolic(nsdf, peak);
        if tau_est <= 0.0 {
            return (0.0, highest);
        }
        (self.sr / tau_est, nsdf[peak].clamp(0.0, 1.0))
    }
}

/// Sub-sample peak location by fitting a parabola to `a[i-1..=i+1]`.
fn parabolic(a: &[f32], i: usize) -> f32 {
    if i == 0 || i + 1 >= a.len() {
        return i as f32;
    }
    let (l, c, r) = (a[i - 1], a[i], a[i + 1]);
    let denom = l - 2.0 * c + r;
    if denom.abs() < 1e-9 {
        return i as f32;
    }
    i as f32 - 0.5 * (r - l) / denom
}

/// Goertzel magnitude of `s` at frequency `f` (Hz), amplitude-normalised.
fn goertzel(s: &[f32], f: f32, sr: f32) -> f32 {
    let w = 2.0 * PI * f / sr;
    let coeff = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0f32, 0.0f32);
    for &x in s {
        let s0 = x + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    let real = s1 - s2 * w.cos();
    let imag = s2 * w.sin();
    (real * real + imag * imag).sqrt() / (s.len() as f32 / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_naming_is_correct() {
        let a4 = note_of(440.0);
        assert_eq!(a4.name, "A");
        assert_eq!(a4.octave, 4);
        assert!(a4.cents.abs() < 0.01);

        let e2 = note_of(82.41);
        assert_eq!(e2.name, "E");
        assert_eq!(e2.octave, 2);
        assert!(e2.cents.abs() < 2.0);

        let c4 = note_of(261.63);
        assert_eq!(c4.name, "C");
        assert_eq!(c4.octave, 4);
    }

    #[test]
    fn flat_note_reads_negative_cents() {
        // A hair below A4 should read as A4, slightly flat.
        let r = note_of(437.0);
        assert_eq!(r.name, "A");
        assert!(r.cents < 0.0, "expected flat, got {}", r.cents);
        assert!(r.cents > -50.0);
    }

    /// Pure sine tones across the neck must be detected within a few cents.
    #[test]
    fn detects_pitch_of_pure_tones() {
        let sr = 48_000.0;
        for &f in &[
            82.41f32, 110.0, 146.83, 196.0, 246.94, 329.63, 440.0, 659.25,
        ] {
            let mut det = TunerDetector::new(sr);
            let tuner = Tuner::new();
            // Feed ~0.2 s so several windows are analysed.
            let block: Vec<f32> = (0..(sr as usize / 5))
                .map(|n| (2.0 * PI * f * n as f32 / sr).sin() * 0.5)
                .collect();
            det.process(&block, &tuner);

            let got = tuner.freq.load(Relaxed);
            assert!(got > 0.0, "no pitch detected for {f} Hz");
            let cents = 1200.0 * (got / f).log2();
            assert!(
                cents.abs() < 10.0,
                "{f} Hz detected as {got:.2} Hz ({cents:.1} cents off)"
            );
        }
    }

    /// Silence (and near-silence) must report no pitch, not a spurious one.
    #[test]
    fn silence_reports_no_pitch() {
        let sr = 48_000.0;
        let mut det = TunerDetector::new(sr);
        let tuner = Tuner::new();
        det.process(&vec![0.0f32; sr as usize / 5], &tuner);
        assert_eq!(tuner.freq.load(Relaxed), 0.0);
        assert_eq!(tuner.clarity.load(Relaxed), 0.0);
    }

    /// The Goertzel spectrum must peak at the bin nearest the played tone.
    #[test]
    fn spectrum_peaks_near_the_played_note() {
        let sr = 48_000.0;
        let f = 220.0;
        let mut det = TunerDetector::new(sr);
        let tuner = Tuner::new();
        let block: Vec<f32> = (0..(sr as usize / 5))
            .map(|n| (2.0 * PI * f * n as f32 / sr).sin() * 0.5)
            .collect();
        det.process(&block, &tuner);

        let (mut best, mut best_mag) = (0usize, 0.0f32);
        for i in 0..SPECTRUM_BINS {
            let m = tuner.spectrum[i].load(Relaxed);
            if m > best_mag {
                best_mag = m;
                best = i;
            }
        }
        let bin_f = spectrum_bin_freq(best);
        // Within a couple of log-spaced bins of the true frequency.
        let cents = 1200.0 * (bin_f / f).log2();
        assert!(
            cents.abs() < 120.0,
            "spectrum peak at {bin_f:.1} Hz, expected near {f} Hz"
        );
    }
}
