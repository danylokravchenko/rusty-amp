pub mod marshall;
pub mod mesa;
pub mod randall;

use crate::dsp::AmpModel;
use crate::dsp::biquad::Biquad;

pub use marshall::Marshall;
pub use mesa::Mesa;
pub use randall::Randall;

// ── Power-amp ↔ speaker interaction ─────────────────────────────────────────

/// Models the way a real power amp "sees" the loudspeaker's impedance curve
/// through its negative-feedback loop.
///
/// A speaker is not a flat resistive load: its impedance has a tall resonant peak
/// near the cabinet's tuning (~80–110 Hz) and rises again through the treble from
/// voice-coil inductance. Because the power amp has a finite output impedance, more
/// drive develops across the speaker exactly where its impedance is high — so the
/// low-frequency resonance blooms and the top end lifts. Crucially this is
/// *dynamic*: as the power supply sags under hard playing the damping factor drops
/// and the low-end resonance opens up further, giving the amp its touch-dependent
/// "give" and three-dimensional low end.
///
/// We tap a resonant band (a 0 dB band-pass at the resonance) and feed back a
/// portion that grows with the sag envelope, plus a static high shelf for the
/// inductive treble rise.
pub(crate) struct SpeakerLoad {
    resonance: Biquad,
    presence: Biquad,
    res_base: f32,
    res_dyn: f32,
}

impl SpeakerLoad {
    /// `fs` resonance frequency, `q` its sharpness, `res_base` the static
    /// resonance amount, `res_dyn` how much more the sag envelope adds, and
    /// `pres_db` the inductive high-shelf lift (at 5 kHz).
    pub fn new(sr: f32, fs: f32, q: f32, res_base: f32, res_dyn: f32, pres_db: f32) -> Self {
        Self {
            resonance: Biquad::bandpass(sr, fs, q),
            presence: Biquad::high_shelf(sr, 5000.0, pres_db),
            res_base,
            res_dyn,
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32, sag: f32) -> f32 {
        let band = self.resonance.process(x);
        let amt = self.res_base + self.res_dyn * sag;
        self.presence.process(x + band * amt)
    }
}

// ── Dynamic "bloom" ─────────────────────────────────────────────────────────

/// Slow envelope follower used to give a gain stage playing dynamics.
///
/// A real tube's operating point drifts under sustained drive (grid-bias
/// excursion / cathode self-bias). Feeding this envelope in as a small DC bias
/// before an asymmetric waveshaper increases even-harmonic content and adds a
/// gentle "give" the harder you play — the touch sensitivity and bloom that make
/// a tube amp feel alive rather than statically clamped. The following
/// inter-stage high-pass removes the injected DC, leaving only the harmonic and
/// compression effect.
pub(crate) struct Bloom {
    env: f32,
    atk: f32,
    rel: f32,
}

impl Bloom {
    pub fn new(sr: f32, atk_ms: f32, rel_ms: f32) -> Self {
        Self {
            env: 0.0,
            atk: 1.0 - (-1.0 / (atk_ms * 0.001 * sr)).exp(),
            rel: 1.0 - (-1.0 / (rel_ms * 0.001 * sr)).exp(),
        }
    }

    #[inline]
    pub fn follow(&mut self, x: f32) -> f32 {
        let a = x.abs();
        let c = if a > self.env { self.atk } else { self.rel };
        self.env += c * (a - self.env);
        self.env
    }
}

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Common interface every amp model must satisfy.
/// All knobs are normalised 0–1.
pub trait Amplifier {
    #[allow(clippy::too_many_arguments)]
    fn process(
        &mut self,
        sample: f32,
        gain: f32,
        bass: f32,
        mid: f32,
        treble: f32,
        presence: f32,
        master: f32,
    ) -> f32;
}

// ── Bank ──────────────────────────────────────────────────────────────────────

/// Owns all amp instances simultaneously so filter state is preserved across
/// model switches (no audible click from zeroed delay lines on switch).
pub struct AmpBank {
    marshall: Marshall,
    mesa: Mesa,
    randall: Randall,
}

impl AmpBank {
    pub fn new(sr: f32) -> Self {
        Self {
            marshall: Marshall::new(sr),
            mesa: Mesa::new(sr),
            randall: Randall::new(sr),
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn process(
        &mut self,
        model: AmpModel,
        sample: f32,
        gain: f32,
        bass: f32,
        mid: f32,
        treble: f32,
        presence: f32,
        master: f32,
    ) -> f32 {
        match model {
            AmpModel::Marshall => self
                .marshall
                .process(sample, gain, bass, mid, treble, presence, master),
            AmpModel::Mesa => self
                .mesa
                .process(sample, gain, bass, mid, treble, presence, master),
            AmpModel::Randall => self
                .randall
                .process(sample, gain, bass, mid, treble, presence, master),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const SR: f32 = 48_000.0;

    /// One amp instance per model, addressed through the `Amplifier` trait so the
    /// sound-quality checks below run identically against all three.
    fn each_amp() -> Vec<(&'static str, Box<dyn Amplifier>)> {
        vec![
            ("Marshall", Box::new(Marshall::new(SR)) as Box<dyn Amplifier>),
            ("Mesa", Box::new(Mesa::new(SR))),
            ("Randall", Box::new(Randall::new(SR))),
        ]
    }

    /// Single-bin DFT magnitude (normalised so a unit sine reads ~1.0).
    fn goertzel(samples: &[f32], f: f32, sr: f32) -> f32 {
        let w = 2.0 * PI * f / sr;
        let coeff = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f32, 0.0f32);
        for &x in samples {
            let s0 = x + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let real = s1 - s2 * w.cos();
        let imag = s2 * w.sin();
        (real * real + imag * imag).sqrt() / (samples.len() as f32 / 2.0)
    }

    /// Push a tone through one amp and collect the steady-state tail (filter and
    /// envelope transients discarded). `amp_amp` is the input sine amplitude.
    #[allow(clippy::too_many_arguments)]
    fn run_tone(
        amp: &mut dyn Amplifier,
        freq: f32,
        amp_amp: f32,
        gain: f32,
        bass: f32,
        mid: f32,
        treble: f32,
        presence: f32,
        master: f32,
    ) -> Vec<f32> {
        let n = SR as usize;
        let warmup = n / 3; // let sag/bloom envelopes and HP filters settle
        let mut out = Vec::with_capacity(n - warmup);
        for i in 0..n {
            let x = (2.0 * PI * freq * i as f32 / SR).sin() * amp_amp;
            let y = amp.process(x, gain, bass, mid, treble, presence, master);
            if i >= warmup {
                out.push(y);
            }
        }
        out
    }

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|&x| x * x).sum::<f32>() / s.len() as f32).sqrt()
    }

    fn mean(s: &[f32]) -> f32 {
        s.iter().sum::<f32>() / s.len() as f32
    }

    /// No amp may produce NaN/Inf or a runaway level across the full sweep of the
    /// gain and master controls — the cheapest guarantee against the worst
    /// "unpleasant sound" of all (a blast of digital noise).
    #[test]
    fn stable_and_bounded_across_control_sweep() {
        for (name, mut amp) in each_amp() {
            let mut max_abs = 0.0f32;
            for &gain in &[0.0, 0.5, 1.0] {
                for &master in &[0.0, 0.5, 1.0] {
                    // hot low-E so the gain stages are genuinely driven
                    for i in 0..(SR as usize / 4) {
                        let x = (2.0 * PI * 82.41 * i as f32 / SR).sin() * 0.9;
                        let y = amp.process(x, gain, 0.7, 0.5, 0.7, 0.6, master);
                        assert!(y.is_finite(), "{name} non-finite at gain={gain} master={master}");
                        max_abs = max_abs.max(y.abs());
                    }
                }
            }
            assert!(max_abs < 4.0, "{name} runaway output: {max_abs}");
        }
    }

    /// The asymmetric tube/FET/silicon waveshapers all inject a DC offset. The
    /// inter-stage and power-amp high-passes exist to strip it; a residual DC
    /// offset wastes headroom and thumps on note transitions. Confirm the
    /// steady-state output is centred on zero even under hard, asymmetric drive.
    #[test]
    fn output_is_dc_free_under_hard_drive() {
        for (name, mut amp) in each_amp() {
            let out = run_tone(&mut *amp, 110.0, 0.8, 0.95, 0.6, 0.5, 0.7, 0.6, 0.7);
            let dc = mean(&out).abs();
            let level = rms(&out).max(1e-6);
            assert!(
                dc < 0.03 * level,
                "{name} has DC offset: |mean|={dc:.4} vs rms={level:.4}"
            );
        }
    }

    /// Driving an amp hard must add harmonics (that is the whole point) but the
    /// energy has to land on the *harmonic series* of the note — musical overtones —
    /// not smear into inharmonic hash from aliasing in the stacked clippers. The
    /// 8× oversampling is what keeps that hash inaudible; this test fails loudly if
    /// the oversampling is ever broken or removed.
    #[test]
    fn distortion_is_harmonic_not_aliased_hash() {
        let f0 = 220.0; // A3
        // Harmonic bins (well below Nyquist) vs. clearly inharmonic probe bins.
        let harmonics: Vec<f32> = (1..=20).map(|k| f0 * k as f32).collect();
        let inharmonic = [130.0, 290.0, 510.0, 1234.0, 2050.0, 3001.0, 5003.0];
        for (name, mut amp) in each_amp() {
            let out = run_tone(&mut *amp, f0, 0.5, 0.95, 0.5, 0.5, 0.7, 0.5, 0.7);

            let h2 = goertzel(&out, f0 * 2.0, SR);
            let h3 = goertzel(&out, f0 * 3.0, SR);
            let fund = goertzel(&out, f0, SR);
            // Genuine distortion: the 2nd/3rd harmonics carry real energy.
            assert!(
                h2 + h3 > 0.1 * fund,
                "{name} barely distorting: h2+h3={:.4}, fund={fund:.4}",
                h2 + h3
            );

            let harm_energy: f32 = harmonics.iter().map(|&f| goertzel(&out, f, SR).powi(2)).sum();
            let alias_energy: f32 =
                inharmonic.iter().map(|&f| goertzel(&out, f, SR).powi(2)).sum();
            assert!(
                alias_energy < 0.02 * harm_energy,
                "{name} aliasing/hash too high: alias={alias_energy:.6} harm={harm_energy:.6}"
            );
        }
    }

    /// A low-E power chord through a high-gain, scooped rig must stay tight: the
    /// inaudible sub-bass below the 82 Hz fundamental (difference-tone "fart") must
    /// remain a small fraction of the musical body harmonics. Mirrors the worst
    /// case the subsonic high-passes in each amp were built to defeat.
    #[test]
    fn power_chord_low_end_stays_tight() {
        let chord = [82.41f32, 123.47, 164.81]; // E2 root + fifth + octave
        for (name, mut amp) in each_amp() {
            let n = SR as usize;
            let warmup = n / 3;
            let mut out = Vec::with_capacity(n - warmup);
            for i in 0..n {
                let t = i as f32 / SR;
                let x: f32 = chord.iter().map(|&f| (2.0 * PI * f * t).sin()).sum::<f32>() * 0.3;
                let y = amp.process(x, 0.93, 0.82, 0.12, 0.86, 0.73, 0.65);
                if i >= warmup {
                    out.push(y);
                }
            }
            let sub = goertzel(&out, 41.0, SR) + goertzel(&out, 55.0, SR);
            let body = goertzel(&out, 164.81, SR) + goertzel(&out, 247.0, SR)
                + goertzel(&out, 330.0, SR);
            assert!(
                sub / body.max(1e-9) < 0.45,
                "{name} low end is farty: sub/body = {:.2}",
                sub / body.max(1e-9)
            );
        }
    }

    /// The tone and presence controls must move the spectrum in the expected
    /// direction — bass up brightens the lows, treble up the highs, presence the
    /// upper-mid air. Uses a small signal so the tone stack is exercised roughly
    /// linearly. Guards against an inverted or dead control shipping a harsh tone.
    #[test]
    fn tone_and_presence_controls_track() {
        // (probe freq, control index: 0=bass 1=treble 2=presence)
        let band = |amp: &mut dyn Amplifier, f: f32, b: f32, t: f32, p: f32| {
            let out = run_tone(amp, f, 0.05, 0.4, b, 0.5, t, p, 0.7);
            goertzel(&out, f, SR)
        };
        for (name, mut amp) in each_amp() {
            let a = &mut *amp;
            assert!(
                band(a, 100.0, 0.9, 0.65, 0.5) > band(a, 100.0, 0.1, 0.65, 0.5),
                "{name} bass control dead/inverted at 100 Hz"
            );
            assert!(
                band(a, 4000.0, 0.5, 0.9, 0.5) > band(a, 4000.0, 0.5, 0.1, 0.5),
                "{name} treble control dead/inverted at 4 kHz"
            );
            assert!(
                band(a, 5000.0, 0.5, 0.65, 0.95) > band(a, 5000.0, 0.5, 0.65, 0.05),
                "{name} presence control dead/inverted at 5 kHz"
            );
        }
    }

    /// At equal settings the three models must sit within a sane loudness window of
    /// each other, so flipping models on stage doesn't jump the volume. The output
    /// trims in each amp exist precisely to enforce this.
    #[test]
    fn amps_are_loudness_matched() {
        let mut levels = Vec::new();
        for (_name, mut amp) in each_amp() {
            // Driven hard — the regime the per-amp output trims are tuned to match.
            let out = run_tone(&mut *amp, 110.0, 0.6, 0.93, 0.5, 0.5, 0.65, 0.5, 0.65);
            levels.push(rms(&out));
        }
        let lo = levels.iter().cloned().fold(f32::INFINITY, f32::min);
        let hi = levels.iter().cloned().fold(0.0, f32::max);
        assert!(
            hi / lo < 2.5,
            "amps not loudness-matched: rms spread {hi:.4}/{lo:.4} = {:.2}x",
            hi / lo
        );
    }
}
