pub mod marshall;
pub mod mesa;
pub mod randall;
pub mod vox;

use crate::dsp::AmpModel;
use crate::dsp::biquad::Biquad;

pub use marshall::Marshall;
pub use mesa::Mesa;
pub use randall::Randall;
pub use vox::Vox;

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

/// Per-stage dynamic grid/cathode bias with an RC recovery time constant — the
/// "live" core of a real 12AX7 stage that a memoryless waveshaper cannot capture.
///
/// In a cathode-biased triode the cathode-bypass cap holds the DC operating point.
/// Hard positive grid excursions draw grid current, which charges the coupling and
/// cathode network and pushes the *average* bias toward cutoff. That shift decays
/// back over an RC time constant (the caps bleeding through the grid-leak resistor).
/// The audible consequences are the two things players feel as "tube give":
///   • **Blocking-distortion bloom** — under a hard transient the stage momentarily
///     biases colder, so gain sags then recovers, swelling the note instead of
///     clamping it flat.
///   • **Dynamic asymmetry** — the moving operating point shifts where on the
///     transfer curve the signal sits, so even-harmonic content grows the harder
///     you dig in and relaxes when you back off (touch sensitivity).
///
/// This runs *at the oversampled rate*, just before the stage's waveshaper, and the
/// DC component of the shift is removed downstream by the inter-stage coupling
/// high-pass — leaving only the dynamic gain/harmonic motion.
pub(crate) struct CathodeBias {
    bias: f32,
    charge: f32,
    recover: f32,
    depth: f32,
    thresh: f32,
}

impl CathodeBias {
    /// `sr` is the rate this is clocked at (the oversampled rate). `charge_ms` is
    /// the (fast) grid-conduction charging time, `recover_ms` the (slow) RC bleed
    /// back to the resting bias, `depth` how far the stored charge shifts the
    /// operating point, and `thresh` the drive level at which grid current starts.
    pub fn new(sr: f32, charge_ms: f32, recover_ms: f32, depth: f32, thresh: f32) -> Self {
        Self {
            bias: 0.0,
            charge: 1.0 - (-1.0 / (charge_ms * 0.001 * sr)).exp(),
            recover: 1.0 - (-1.0 / (recover_ms * 0.001 * sr)).exp(),
            depth,
            thresh,
        }
    }

    /// Apply the dynamic bias shift to a stage input already scaled to the
    /// waveshaper's drive range. Returns the bias-shifted value to clip.
    #[inline]
    pub fn shift(&mut self, x: f32) -> f32 {
        // Grid current flows only on hard positive excursions past the conduction
        // threshold. It charges the cap fast and bleeds away slowly (RC recovery).
        let conduction = (x - self.thresh).max(0.0);
        let c = if conduction > self.bias {
            self.charge
        } else {
            self.recover
        };
        self.bias += c * (conduction - self.bias);
        // Stored charge biases the stage toward cutoff, reducing gain on the
        // samples that follow — the blocking-distortion "give".
        x - self.bias * self.depth
    }
}

/// Output-transformer character: low-frequency core saturation plus a trace of
/// class-AB crossover content.
///
/// A guitar output transformer is not a clean gain block. At high flux — which on
/// a transformer means *low frequencies* — the core saturates, gently compressing
/// and rounding the bottom end. That is the woolly, breathing low end of a cranked
/// power amp, and crucially it is frequency-selective: the highs (low flux) stay
/// linear, so only the lows compress. Separately, a push-pull output pair handing
/// the signal off at the zero crossing leaves a small amount of crossover content
/// that adds odd-harmonic complexity and "bite" without an audible hard kink.
pub(crate) struct OutputTransformer {
    lf: Biquad,
    drive: f32,
    xover: f32,
}

impl OutputTransformer {
    /// `lf_corner` splits the saturating low-frequency flux from the linear highs,
    /// `drive` sets how hard the core is pushed (higher = more LF compression), and
    /// `xover` the depth of the class-AB crossover content.
    pub fn new(sr: f32, lf_corner: f32, drive: f32, xover: f32) -> Self {
        Self {
            lf: Biquad::lowpass(sr, lf_corner, 0.707),
            drive,
            xover,
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        // Split the low-frequency flux (which saturates the core) from the highs.
        let low = self.lf.process(x);
        let high = x - low;
        // Core saturation acts on the lows only; normalised so small-signal gain is
        // unity (tanh(low·d)/d → low as low → 0) and only larger flux compresses.
        let y = (low * self.drive).tanh() / self.drive + high;
        // A touch of crossover: gain dips slightly through the zero-crossing handoff
        // and vanishes for larger swings, so it adds odd-harmonic bite, not a kink.
        let cross = self.xover * y * (-(y * y) * 25.0).exp();
        y - cross
    }
}

/// Treble-bleed ("bright") cap bridging the gain pot, as on a Marshall-style
/// preamp.
///
/// A small cap across the gain control passes high frequencies around the wiper.
/// Its effect is strongest with the pot turned down (more resistance for the cap to
/// bleed across) and washes out as the pot is opened up. Musically this adds
/// sparkle and cut at low-to-moderate gain that tightens as the amp is cranked —
/// the reason a Marshall stays articulate at the edge of breakup.
pub(crate) struct BrightCap {
    hp: Biquad,
    amount: f32,
}

impl BrightCap {
    /// `corner` is the cap's high-pass corner (above which it bleeds), `amount` the
    /// peak amount of high end injected (at gain = 0).
    pub fn new(sr: f32, corner: f32, amount: f32) -> Self {
        Self {
            hp: Biquad::highpass(sr, corner, 0.707),
            amount,
        }
    }

    /// Inject the bright-cap highs, weighted by how far the gain pot is *down*.
    #[inline]
    pub fn process(&mut self, x: f32, gain: f32) -> f32 {
        let highs = self.hp.process(x);
        x + highs * self.amount * (1.0 - gain)
    }
}

/// Shared preamp front end: a fixed DC blocker followed by the model's input
/// high-pass.
///
/// Every model opens the same way — strip any DC at ~10 Hz, then a model-specific
/// subsonic/rumble high-pass before the gain stages. Only the input-HP corner
/// differs (tube amps sit lower, the solid-state Randall tighter), so that is the
/// one knob the constructor takes.
pub(crate) struct FrontEnd {
    dc_block: Biquad,
    input_hp: Biquad,
}

impl FrontEnd {
    pub fn new(sr: f32, input_hp_hz: f32) -> Self {
        Self {
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            input_hp: Biquad::highpass(sr, input_hp_hz, 0.707),
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.input_hp.process(self.dc_block.process(x))
    }
}

/// Structural voicing balance applied after the tone stack: a low shelf that
/// restores low-mid body and a high shelf that tames the tone stack's treble-
/// forward tilt, so notes stay even in level across the neck.
///
/// All three models need this same body-up / tilt-down pair (the gain-stage
/// high-passes and peak-normalised tone stacks otherwise leave the upper register
/// blasting out); only the corner frequencies and depths are voiced per model.
pub(crate) struct VoiceBalance {
    body: Biquad,
    tilt: Biquad,
}

impl VoiceBalance {
    pub fn new(sr: f32, body_hz: f32, body_db: f32, tilt_hz: f32, tilt_db: f32) -> Self {
        Self {
            body: Biquad::low_shelf(sr, body_hz, body_db),
            tilt: Biquad::high_shelf(sr, tilt_hz, tilt_db),
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.tilt.process(self.body.process(x))
    }
}

/// Tracks the value a control was last at so an expensive coefficient recompute
/// only fires when the knob actually moves. Starts "dirty" (NaN), so the first
/// real call always recomputes.
pub(crate) struct Cached {
    last: f32,
}

impl Cached {
    pub fn new() -> Self {
        Self { last: f32::NAN }
    }

    /// Returns `true` (and latches the new value) when `v` has moved beyond the
    /// smoothing epsilon since the last latched value. The initial NaN forces the
    /// first call to report changed, syncing coefficients to the real control value.
    #[inline]
    pub fn changed(&mut self, v: f32) -> bool {
        if self.last.is_nan() || (v - self.last).abs() > 0.001 {
            self.last = v;
            true
        } else {
            false
        }
    }
}

/// The three-knob counterpart of [`Cached`] for the bass/mid/treble tone stack,
/// whose coefficients are recomputed as a set whenever *any* of the three moves.
pub(crate) struct ToneCache {
    bass: f32,
    mid: f32,
    treble: f32,
}

impl ToneCache {
    pub fn new() -> Self {
        Self {
            bass: f32::NAN,
            mid: f32::NAN,
            treble: f32::NAN,
        }
    }

    #[inline]
    pub fn changed(&mut self, bass: f32, mid: f32, treble: f32) -> bool {
        let moved = self.bass.is_nan()
            || (bass - self.bass).abs() > 0.001
            || (mid - self.mid).abs() > 0.001
            || (treble - self.treble).abs() > 0.001;
        if moved {
            self.bass = bass;
            self.mid = mid;
            self.treble = treble;
        }
        moved
    }
}

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

/// Owns all amp instances simultaneously so filter state is preserved across
/// model switches (no audible click from zeroed delay lines on switch).
pub struct AmpBank {
    marshall: Marshall,
    mesa: Mesa,
    randall: Randall,
    vox: Vox,
}

impl AmpBank {
    pub fn new(sr: f32) -> Self {
        Self {
            marshall: Marshall::new(sr),
            mesa: Mesa::new(sr),
            randall: Randall::new(sr),
            vox: Vox::new(sr),
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
            AmpModel::Vox => self
                .vox
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
            (
                "Marshall",
                Box::new(Marshall::new(SR)) as Box<dyn Amplifier>,
            ),
            ("Mesa", Box::new(Mesa::new(SR))),
            ("Randall", Box::new(Randall::new(SR))),
            ("Vox", Box::new(Vox::new(SR))),
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
                        assert!(
                            y.is_finite(),
                            "{name} non-finite at gain={gain} master={master}"
                        );
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

            let harm_energy: f32 = harmonics
                .iter()
                .map(|&f| goertzel(&out, f, SR).powi(2))
                .sum();
            let alias_energy: f32 = inharmonic
                .iter()
                .map(|&f| goertzel(&out, f, SR).powi(2))
                .sum();
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
            let body =
                goertzel(&out, 164.81, SR) + goertzel(&out, 247.0, SR) + goertzel(&out, 330.0, SR);
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

    // ── New dynamic-realism features ──────────────────────────────────────────
    //
    // The three additions below — dynamic cathode-bias, output-transformer
    // saturation/crossover, and the gain-pot bright cap — exist to make the amp
    // *react* the way real iron and tubes do rather than sit as a static transfer
    // curve. Each is unit-tested in isolation (so a regression points straight at
    // the offending block) and then again through a whole amp (so we know the wiring
    // and tuning actually deliver the effect at the controls a player turns).

    // — Dynamic cathode bias (blocking-distortion bloom / touch) ———————————————

    /// Below grid conduction the stage must be perfectly transparent: a clean, quiet
    /// signal that never swings the grid into current must pass untouched, with no
    /// bias build-up colouring it. This is what keeps low-level playing clear instead
    /// of permanently "ducked".
    #[test]
    fn cathode_bias_is_transparent_below_conduction() {
        let mut cb = CathodeBias::new(SR, 1.5, 45.0, 0.3, 1.0);
        for i in 0..(SR as usize / 10) {
            let x = (2.0 * PI * 200.0 * i as f32 / SR).sin() * 0.8; // peaks 0.8 < thresh 1.0
            let y = cb.shift(x);
            assert!(
                (y - x).abs() < 1e-6,
                "cathode bias altered a signal that never reaches grid conduction"
            );
        }
    }

    /// The defining dynamic behaviour: when drive suddenly exceeds grid conduction
    /// the bias charges up and pulls the operating point colder, so the stage gain
    /// *sags in* over the first few milliseconds (blocking-distortion bloom) instead
    /// of clamping flat instantly. We measure the positive-peak envelope right at
    /// onset vs. once the bias has settled and require a clear droop.
    #[test]
    fn cathode_bias_blooms_under_a_hard_transient() {
        let mut cb = CathodeBias::new(SR, 1.5, 45.0, 0.3, 1.0);
        let n = SR as usize / 10; // 100 ms
        let mut peaks = Vec::with_capacity(n);
        for i in 0..n {
            let x = (2.0 * PI * 200.0 * i as f32 / SR).sin() * 2.0; // peaks well past thresh
            peaks.push(cb.shift(x));
        }
        let win = SR as usize / 100; // 10 ms
        let early = peaks[..win].iter().cloned().fold(f32::MIN, f32::max);
        let late = peaks[n - win..].iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            early > late + 0.05,
            "cathode bias shows no blocking-distortion sag: early peak {early:.3} late {late:.3}"
        );
    }

    /// The stored charge must bleed away over the RC recovery time so the *next*
    /// note starts from the resting bias — otherwise hard playing would leave the
    /// amp permanently compressed. After a loud burst and a recovery window, a quiet
    /// probe must again pass essentially untouched.
    #[test]
    fn cathode_bias_recovers_after_the_drive_stops() {
        let mut cb = CathodeBias::new(SR, 1.5, 45.0, 0.3, 1.0);
        for i in 0..(SR as usize / 10) {
            let x = (2.0 * PI * 200.0 * i as f32 / SR).sin() * 2.0; // load it up
            cb.shift(x);
        }
        for _ in 0..(SR as usize / 4) {
            cb.shift(0.0); // 250 ms of silence — RC recovery (~45 ms) completes
        }
        let y = cb.shift(0.5); // below conduction
        assert!(
            (y - 0.5).abs() < 0.01,
            "cathode bias never recovered: probe {y:.4} (expected ~0.5)"
        );
    }

    /// The bias shift is one-sided (it pulls the operating point toward cutoff), so
    /// under symmetric hard drive it skews the waveform's average negative. That DC
    /// skew (stripped later by the inter-stage HP) is exactly the dynamic asymmetry
    /// that breeds the even-harmonic warmth a static shaper can't.
    #[test]
    fn cathode_bias_skews_the_operating_point_under_drive() {
        let mut cb = CathodeBias::new(SR, 1.5, 45.0, 0.3, 1.0);
        let n = SR as usize / 5;
        let warm = n / 2;
        let mut out = Vec::with_capacity(n - warm);
        for i in 0..n {
            let x = (2.0 * PI * 200.0 * i as f32 / SR).sin() * 2.0;
            let y = cb.shift(x);
            if i >= warm {
                out.push(y);
            }
        }
        let m = mean(&out);
        assert!(
            m < -0.02,
            "cathode bias did not skew the operating point under hard drive: mean {m:.4}"
        );
    }

    // — Output transformer (LF core saturation + crossover) ————————————————————

    /// The transformer must compress the *lows* (high core flux) while leaving the
    /// *highs* (low flux) essentially linear — the frequency-selective give that
    /// makes a cranked power amp woolly on the bottom but not smeared on top. We
    /// compare effective gain quiet-vs-loud at a low and a high frequency.
    #[test]
    fn output_transformer_compresses_lows_not_highs() {
        let eff_gain = |freq: f32, amp_in: f32| -> f32 {
            let mut ot = OutputTransformer::new(SR, 150.0, 1.5, 0.04);
            let n = SR as usize / 4;
            let warm = n / 2;
            let mut out = Vec::with_capacity(n - warm);
            for i in 0..n {
                let x = (2.0 * PI * freq * i as f32 / SR).sin() * amp_in;
                let y = ot.process(x);
                if i >= warm {
                    out.push(y);
                }
            }
            goertzel(&out, freq, SR) / amp_in
        };
        let low_quiet = eff_gain(60.0, 0.1);
        let low_loud = eff_gain(60.0, 1.5);
        assert!(
            low_loud < low_quiet * 0.85,
            "OT lows not compressing: quiet {low_quiet:.3} → loud {low_loud:.3}"
        );
        let high_quiet = eff_gain(4000.0, 0.1);
        let high_loud = eff_gain(4000.0, 1.5);
        assert!(
            high_loud > high_quiet * 0.9,
            "OT is compressing highs (should pass clean): quiet {high_quiet:.3} → loud {high_loud:.3}"
        );
    }

    /// A hard low note through the transformer must grow genuine harmonic
    /// complexity (core saturation → odd harmonics, crossover → bite) while staying
    /// finite and bounded — woolly warmth, not a fizzy mess.
    #[test]
    fn output_transformer_adds_low_harmonics_and_is_finite() {
        let mut ot = OutputTransformer::new(SR, 150.0, 1.6, 0.05);
        let freq = 80.0;
        let n = SR as usize / 4;
        let warm = n / 2;
        let mut out = Vec::with_capacity(n - warm);
        for i in 0..n {
            let x = (2.0 * PI * freq * i as f32 / SR).sin() * 1.5;
            let y = ot.process(x);
            assert!(y.is_finite() && y.abs() < 4.0, "OT unstable: {y}");
            if i >= warm {
                out.push(y);
            }
        }
        let fund = goertzel(&out, freq, SR);
        let h2 = goertzel(&out, freq * 2.0, SR);
        let h3 = goertzel(&out, freq * 3.0, SR);
        assert!(
            h2 + h3 > 0.02 * fund,
            "OT added no harmonic complexity: h2+h3={:.4} fund={fund:.4}",
            h2 + h3
        );
    }

    // — Bright cap (treble-bleed across the gain pot) ——————————————————————————

    /// The cap bleeds treble around the gain wiper, strongest with the pot down and
    /// gone when it's wide open — and it must only touch the highs, never the lows
    /// (the cap blocks them). Verified directly on the building block.
    #[test]
    fn bright_cap_adds_treble_only_when_the_pot_is_down() {
        let level = |gain: f32, freq: f32| -> f32 {
            let mut bc = BrightCap::new(SR, 2000.0, 0.2);
            let n = SR as usize / 4;
            let warm = n / 2;
            let mut out = Vec::with_capacity(n - warm);
            for i in 0..n {
                let x = (2.0 * PI * freq * i as f32 / SR).sin();
                let y = bc.process(x, gain);
                if i >= warm {
                    out.push(y);
                }
            }
            goertzel(&out, freq, SR)
        };
        // Highs (above the cap corner): boosted at low gain, ~unity wide open.
        let treble_low_gain = level(0.0, 4000.0);
        let treble_high_gain = level(1.0, 4000.0);
        assert!(
            treble_low_gain > treble_high_gain * 1.05,
            "bright cap adds no treble at low gain: {treble_low_gain:.3} vs {treble_high_gain:.3}"
        );
        assert!(
            (treble_high_gain - 1.0).abs() < 0.05,
            "bright cap not ~unity with the pot wide open: {treble_high_gain:.3}"
        );
        // Lows (below the corner): the cap blocks them at any setting.
        let bass_low_gain = level(0.0, 100.0);
        assert!(
            (bass_low_gain - 1.0).abs() < 0.1,
            "bright cap is leaking lows: {bass_low_gain:.3}"
        );
    }

    // — Integration: the features reach the player's controls ——————————————————

    /// One instance per tube amp (Marshall + Mesa + Vox), the models that carry the
    /// triode/transformer/bright-cap chain. The Randall is solid-state and is
    /// deliberately left out of these — it has no output transformer or triode stage
    /// to model.
    fn tube_amps() -> Vec<(&'static str, Box<dyn Amplifier>)> {
        vec![
            (
                "Marshall",
                Box::new(Marshall::new(SR)) as Box<dyn Amplifier>,
            ),
            ("Mesa", Box::new(Mesa::new(SR))),
            ("Vox", Box::new(Vox::new(SR))),
        ]
    }

    /// Through a whole amp, the bright cap must make low-gain settings audibly
    /// brighter than cranked ones: the treble-to-mid tilt, measured small-signal so
    /// the gain stages stay roughly linear, must fall as the gain pot is opened.
    #[test]
    fn bright_cap_brightens_low_gain_settings() {
        let tilt = |amp: &mut dyn Amplifier, gain: f32| -> f32 {
            let hi = run_tone(amp, 4000.0, 0.02, gain, 0.5, 0.5, 0.5, 0.5, 0.6);
            let lo = run_tone(amp, 300.0, 0.02, gain, 0.5, 0.5, 0.5, 0.5, 0.6);
            goertzel(&hi, 4000.0, SR) / goertzel(&lo, 300.0, SR).max(1e-9)
        };
        for (name, mut amp) in tube_amps() {
            let a = &mut *amp;
            let low_gain = tilt(a, 0.1);
            let high_gain = tilt(a, 0.9);
            assert!(
                low_gain > high_gain * 1.05,
                "{name}: bright cap not brightening low gain (tilt {low_gain:.3} vs {high_gain:.3})"
            );
        }
    }

    /// The dynamic stages must make the amp *touch sensitive*: digging in harder
    /// should bloom more even-harmonic warmth than playing softly, at the same
    /// settings. Measured at a moderate gain where the stage is responsive (not
    /// already pinned), so the growth comes from the dynamics, not just more static
    /// clipping. This is the single best proxy for "alive, not artificial".
    #[test]
    fn tube_amps_are_touch_sensitive() {
        let h2_over_h1 = |amp: &mut dyn Amplifier, drive_in: f32| -> f32 {
            let out = run_tone(amp, 150.0, drive_in, 0.3, 0.5, 0.5, 0.6, 0.5, 0.6);
            goertzel(&out, 300.0, SR) / goertzel(&out, 150.0, SR).max(1e-9)
        };
        for (name, mut amp) in tube_amps() {
            let a = &mut *amp;
            let soft = h2_over_h1(a, 0.05);
            let hard = h2_over_h1(a, 0.5);
            assert!(
                hard > soft * 1.05,
                "{name}: not touch sensitive (even-harmonic h2/h1 soft {soft:.3} → hard {hard:.3})"
            );
        }
    }
}
