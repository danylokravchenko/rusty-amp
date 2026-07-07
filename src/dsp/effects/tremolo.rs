use std::f32::consts::TAU;

/// Stereo tremolo / vibrato — one LFO driving two kinds of modulation, blended by
/// the MODE knob.
///
/// The two effects are cousins: **tremolo** modulates *amplitude* (the signal
/// swells and ducks — Fender-amp "shimmer", or, squared off, a choppy helicopter
/// pulse), while **vibrato** modulates *pitch* (a short LFO-swept delay line warps
/// the note up and down — the Boss VB-2 wobble). Sharing one LFO lets a single
/// pedal morph between them: MODE 0 = pure tremolo, MODE 1 = pure vibrato, and the
/// middle blends both for a seasick, combined movement.
///
/// Sits in the post-cab stereo rack after the other modulations (flanger, chorus,
/// phaser), just before the delay — the last bit of *movement* applied to the
/// finished tone, ahead of the time-based ambience. Both channels share the LFO,
/// so the pulse/wobble is coherent across the stereo image and the cab's existing
/// L/R decorrelation is preserved.
///
/// Knob ranges (all normalised 0–1):
///   RATE  → LFO speed, 0.5–14 Hz (exponential).
///   DEPTH → modulation intensity (amplitude swing and/or pitch swing).
///   SHAPE → LFO waveform, sine → soft square (choppiness of the tremolo).
///   MODE  → amplitude (tremolo) ↔ pitch (vibrato) blend.
pub struct Tremolo {
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    write: usize,
    phase: f32,
    sr: f32,
}

/// Centre delay of the vibrato tap. The pitch swing modulates around this, so it
/// must sit above `SWING_MS` to keep the read head behind the write head.
const CENTER_MS: f32 = 4.0;
/// Maximum ± pitch swing the DEPTH knob sweeps around `CENTER_MS`.
const SWING_MS: f32 = 3.0;
/// Buffer headroom above the deepest possible delay (`CENTER_MS + SWING_MS`).
const MAX_MS: f32 = 8.0;

impl Tremolo {
    pub fn new(sr: f32) -> Self {
        let len = (sr * MAX_MS / 1000.0) as usize + 2;
        Self {
            buf_l: vec![0.0; len],
            buf_r: vec![0.0; len],
            write: 0,
            phase: 0.0,
            sr,
        }
    }

    /// Linear-interpolated read `delay` samples behind the write head.
    #[inline]
    fn read(buf: &[f32], write: usize, delay: f32) -> f32 {
        let len = buf.len();
        let d = delay.clamp(1.0, (len - 2) as f32);
        let i0 = d.floor() as usize;
        let frac = d - i0 as f32;
        let a = (write + len - i0) % len;
        let b = (write + len - i0 - 1) % len;
        buf[a] * (1.0 - frac) + buf[b] * frac
    }

    #[inline]
    pub fn process(
        &mut self,
        l: f32,
        r: f32,
        rate: f32,
        depth: f32,
        shape: f32,
        mode: f32,
    ) -> (f32, f32) {
        // LFO advances once per sample; exponential map puts the slow, musical rates
        // across most of the knob's travel and the fast helicopter chop up top.
        let rate_hz = 0.5 * 28.0_f32.powf(rate.clamp(0.0, 1.0));
        self.phase = (self.phase + rate_hz / self.sr).fract();
        let sine = (self.phase * TAU).sin();

        let depth = depth.clamp(0.0, 1.0);
        let mode = mode.clamp(0.0, 1.0);
        // MODE splits the DEPTH between the two modulation kinds.
        let amp_depth = depth * (1.0 - mode);
        let pitch_depth = depth * mode;

        // Vibrato: read a delay tap whose length wobbles with the LFO. When
        // `pitch_depth` is 0 (pure tremolo) the tap is a fixed short delay — an
        // inaudible latency, no pitch movement.
        let del_ms = CENTER_MS + pitch_depth * SWING_MS * sine;
        let del = del_ms * self.sr / 1000.0;
        let wet_l = Self::read(&self.buf_l, self.write, del);
        let wet_r = Self::read(&self.buf_r, self.write, del);

        self.buf_l[self.write] = l;
        self.buf_r[self.write] = r;
        let len = self.buf_l.len();
        self.write = (self.write + 1) % len;

        // Tremolo: a shaped LFO ducks the amplitude between `1 - amp_depth` and 1.
        let trem = shape_wave(sine, shape);
        let gain = 1.0 - amp_depth * 0.5 * (1.0 - trem);

        (wet_l * gain, wet_r * gain)
    }
}

/// LFO waveform morph: `shape` 0 = pure sine, 1 = soft square. The soft square is a
/// normalised `tanh` of an overdriven sine, so the tremolo goes from a smooth swell
/// to a choppy on/off pulse without the clicks a hard square edge would inject.
#[inline]
fn shape_wave(sine: f32, shape: f32) -> f32 {
    let shape = shape.clamp(0.0, 1.0);
    let k = 1.0 + shape * 12.0;
    let sq = (sine * k).tanh() / k.tanh();
    sine * (1.0 - shape) + sq * shape
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const SR: f32 = 48_000.0;

    /// Single-bin amplitude via Goertzel (a unit sine reads ~1.0 over whole cycles),
    /// in `f64` so the poles on the unit circle don't lose the bin over long windows.
    fn goertzel(samples: &[f32], f: f32, sr: f32) -> f32 {
        let w = 2.0 * std::f64::consts::PI * f as f64 / sr as f64;
        let coeff = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for &x in samples {
            let s0 = x as f64 + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let real = s1 - s2 * w.cos();
        let imag = s2 * w.sin();
        ((real * real + imag * imag).sqrt() / (samples.len() as f64 / 2.0)) as f32
    }

    /// The peak-to-peak swing of the output envelope, sampled in short windows —
    /// large under tremolo (amplitude moving), small when the level is steady.
    fn envelope_range(mode: f32, depth: f32) -> f32 {
        let mut t = Tremolo::new(SR);
        let mut peaks = Vec::new();
        let mut peak = 0.0f32;
        for n in 0..(SR as usize * 2) {
            let x = (2.0 * PI * 300.0 * n as f32 / SR).sin();
            let (l, _r) = t.process(x, x, 0.6, depth, 0.0, mode);
            if n > SR as usize {
                peak = peak.max(l.abs());
                if n % 200 == 0 {
                    peaks.push(peak);
                    peak = 0.0;
                }
            }
        }
        let hi = peaks.iter().cloned().fold(0.0f32, f32::max);
        let lo = peaks.iter().cloned().fold(f32::INFINITY, f32::min);
        hi - lo
    }

    /// Fraction of a steady sine's energy still on its own bin — near 1.0 when the
    /// pitch is stable, dropping as vibrato FM smears energy into sidebands.
    fn fundamental_fraction(mode: f32, depth: f32) -> f32 {
        let mut t = Tremolo::new(SR);
        let f0 = 1000.0;
        let mut out = Vec::new();
        for n in 0..(SR as usize * 2) {
            let x = (2.0 * PI * f0 * n as f32 / SR).sin();
            let (l, _r) = t.process(x, x, 0.6, depth, 0.0, mode);
            if n > SR as usize {
                out.push(l);
            }
        }
        let rms = (out.iter().map(|&s| (s as f64) * (s as f64)).sum::<f64>() / out.len() as f64)
            .sqrt()
            .max(1e-9);
        (goertzel(&out, f0, SR) as f64 / (rms * std::f64::consts::SQRT_2)) as f32
    }

    /// Extreme settings must stay finite and bounded — the amplitude gain never
    /// exceeds unity and the vibrato tap is a delayed copy, so the peak can't grow.
    #[test]
    fn finite_and_bounded_under_extreme_settings() {
        let mut t = Tremolo::new(SR);
        let mut max_abs = 0.0f32;
        for n in 0..(SR as usize) {
            let x = (2.0 * PI * 220.0 * n as f32 / SR).sin() * 0.9;
            let (l, r) = t.process(x, x, 1.0, 1.0, 1.0, 0.5);
            assert!(l.is_finite() && r.is_finite(), "non-finite at {n}");
            max_abs = max_abs.max(l.abs()).max(r.abs());
        }
        assert!(max_abs < 1.5, "tremolo output out of bounds: {max_abs}");
    }

    /// Tremolo (MODE 0): DEPTH must open up an amplitude swing — a deep setting
    /// modulates the envelope, a zero setting leaves it steady.
    #[test]
    fn tremolo_depth_modulates_amplitude() {
        assert!(
            envelope_range(0.0, 0.9) > envelope_range(0.0, 0.0) + 0.2,
            "tremolo depth did not modulate the amplitude"
        );
    }

    /// Vibrato (MODE 1): DEPTH must FM the pitch — a deep setting spreads energy off
    /// the fundamental into sidebands, so its bin holds a smaller fraction than a
    /// zero (constant-delay) setting does.
    #[test]
    fn vibrato_depth_spreads_the_spectrum() {
        let deep = fundamental_fraction(1.0, 0.9);
        let flat = fundamental_fraction(1.0, 0.0);
        assert!(
            deep < flat * 0.9,
            "vibrato depth did not spread the spectrum: deep {deep:.3} vs flat {flat:.3}"
        );
    }

    /// The MODE knob must actually select which modulation runs: tremolo moves the
    /// amplitude but keeps the pitch stable; vibrato moves the pitch but keeps the
    /// level steady.
    #[test]
    fn mode_selects_tremolo_versus_vibrato() {
        // Tremolo end: big amplitude swing, fundamental essentially intact.
        assert!(
            envelope_range(0.0, 0.9) > envelope_range(1.0, 0.9) * 2.0,
            "tremolo end did not move the amplitude more than the vibrato end"
        );
        // Vibrato end: fundamental spread more than at the tremolo end.
        assert!(
            fundamental_fraction(1.0, 0.9) < fundamental_fraction(0.0, 0.9),
            "vibrato end did not spread the pitch more than the tremolo end"
        );
    }

    /// The SHAPE knob must sharpen the tremolo LFO from a smooth sine toward a square
    /// on/off chop — the shaped wave spends more time near its extremes, so the
    /// envelope's mean sits further from the mid-level swell of a pure sine.
    #[test]
    fn shape_sharpens_the_tremolo_waveform() {
        let flat_top = |shape: f32| {
            let mut t = Tremolo::new(SR);
            let mut near_extreme = 0u32;
            let mut total = 0u32;
            for n in 0..(SR as usize) {
                // DC input so the output *is* the tremolo gain envelope.
                let (l, _r) = t.process(1.0, 1.0, 0.4, 1.0, shape, 0.0);
                if n > SR as usize / 2 {
                    total += 1;
                    if !(0.15..=0.85).contains(&l) {
                        near_extreme += 1;
                    }
                }
            }
            near_extreme as f32 / total as f32
        };
        assert!(
            flat_top(1.0) > flat_top(0.0) * 1.3,
            "shape knob did not sharpen the tremolo waveform"
        );
    }
}
