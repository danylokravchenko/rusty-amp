use super::param_changed;
use crate::dsp::biquad::Biquad;
use crate::dsp::oversample::Oversampler8;

/// Boss ML-2 Metal Core simulation — an ultra-high-gain distortion voiced for
/// modern metal: far more gain than the DS-1, a scooped mid character, and a
/// powerful active two-band (Low/High) EQ instead of a single tone control.
///
/// Signal path:
///   DC block → input HP → fixed mid-scoop → [8× OS: pre-clip HP → pre-clip LP →
///   two cascaded soft-clip stages] → post-clip HP → active Low shelf →
///   active High shelf → post-clip LP → level
///
/// Character & authenticity:
///   • The ML-2 is a *tight, brutal* pedal designed to make any amp sound metal on
///     its own. Its gain comes from **two cascaded clipping stages**: the first
///     saturates, the second thickens it into the long, singing, compressed sustain
///     metal rhythm and lead lean on. But driven flat-out the cascade squares the
///     wave into a fizzy, all-odd buzz that turns chords to mud — *and* its enormous
///     small-signal gain amplifies the hiss and hum riding under the notes into a
///     wash of "noise". So both stages are kept sane (first stage ≤22×, was 101×;
///     second push 1.5×, was 3×): a normal note still slams both stages to the rail,
///     so loud saturation and output level are unchanged, but the quiet noise floor
///     — amplified only by the small-signal slope — drops ~9 dB, and decays clean up
///     dynamically instead of hanging as a buzzy, over-compressed tail.
///   • A **fixed mid-scoop** (~650 Hz) is baked into the voice — the ML-2's scooped,
///     aggressive tonality — but kept moderate so the tone stays articulate rather
///     than hollow. The player's own scoop lives in the active EQ on top.
///   • Tightened on both sides of the clipper like the DS-1: a pre-clip high-pass
///     trims low-mud before the massive gain, a **pre-clip low-pass** rounds the top
///     off the wave so a chord's upper harmonics don't intermodulate into fizz, and
///     a post-clip high-pass strips the woof the clipper generates, so the low end
///     stays defined for palm-mute chug.
///   • Genuine 2nd-harmonic warmth comes from **asymmetric rail levels** (the
///     negative half saturates toward −1.3 vs +1.0), which survive the cascade's
///     hard saturation — unlike a DC bias, which just washes out to a DC offset the
///     post-clip HP removes. A purely-symmetric clipper makes only the clinical odd
///     buzz; the tiny DC this asymmetry leaves is removed by the post-clip high-pass.
///   • The **active Low/High EQ** is a wide-range shelving pair (±15 dB): the ML-2's
///     EQ is powerful enough to reshape the whole voice, from a bass-heavy wall to a
///     bright, cutting lead. 8× oversampling keeps the aggressive clip harmonics from
///     aliasing into digital fizz.
pub struct MetalCore {
    sr: f32,
    dc_block: Biquad,
    input_hp: Biquad,
    // Fixed pre-clip mid-scoop — the ML-2's scooped metal voice.
    mid_scoop: Biquad,
    os: Oversampler8,
    // Pre-clip HP at 8× rate — tightens the low end before the cascaded gain.
    pre_clip_hp: Biquad,
    // Pre-clip LP at 8× rate — rounds the top off the wave *before* the clipper so
    // the cascade generates a shorter harmonic ladder: much of the fizzy top-octave
    // buzz and high-order intermod is never created in the first place, the
    // studio-console way to keep high gain smooth rather than brittle.
    pre_clip_lp: Biquad,
    // Post-clip HP (base rate) — strips the blubber the clipper generates and the
    // small DC the clip asymmetry leaves.
    post_clip_hp: Biquad,
    // Active two-band EQ (base rate): wide-range low + high shelves.
    low_shelf: Biquad,
    high_shelf: Biquad,
    last_low: f32,
    last_high: f32,
    // Post-clip LP (base rate) — tames the harsh clip fizz above the useful range.
    post_clip_lp: Biquad,
}

impl MetalCore {
    pub fn new(sr: f32) -> Self {
        let sr8 = sr * 8.0;
        let mut m = Self {
            sr,
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            // 90 Hz input HP: trims sub-bass before the huge gain so the low end
            // stays tight instead of flubbing out.
            input_hp: Biquad::highpass(sr, 90.0, 0.707),
            // −3.5 dB around 650 Hz: the ML-2's scooped voice, kept moderate so the
            // tone stays articulate rather than hollow.
            mid_scoop: Biquad::peak_eq(sr, 650.0, 0.8, -3.5),
            os: Oversampler8::new(sr),
            // 150 Hz pre-clip HP: keeps the cascade from turning low notes to mush.
            pre_clip_hp: Biquad::highpass(sr8, 150.0, 0.707),
            // 3.8 kHz pre-clip LP: rounds the top off the wave before the huge gain
            // so a chord's upper harmonics don't intermodulate into fizz, while
            // leaving the voice its metal cut. (It can't shorten the ladder the
            // cascade makes below it — that's what the lower gain is for.)
            pre_clip_lp: Biquad::lowpass(sr8, 3800.0, 0.707),
            // 120 Hz post-clip HP: keeps the low E present but strips the woof and
            // the clip asymmetry's DC.
            post_clip_hp: Biquad::highpass(sr, 120.0, 0.707),
            low_shelf: Biquad::low_shelf(sr, 120.0, 0.0),
            high_shelf: Biquad::high_shelf(sr, 3200.0, 0.0),
            last_low: -1.0,
            last_high: -1.0,
            // 7 kHz post-clip LP: keeps the bite but rolls the brittle fizz off
            // before the amp's gain stage.
            post_clip_lp: Biquad::lowpass(sr, 7000.0, 0.707),
        };
        m.update_low(0.5);
        m.update_high(0.5);
        m
    }

    fn update_low(&mut self, low: f32) {
        // Active low shelf @ 120 Hz, ±15 dB; centre (0.5) is flat.
        self.low_shelf = Biquad::low_shelf(self.sr, 120.0, (low - 0.5) * 30.0);
        self.last_low = low;
    }

    fn update_high(&mut self, high: f32) {
        // Active high shelf @ 3.2 kHz, ±15 dB; centre (0.5) is flat.
        self.high_shelf = Biquad::high_shelf(self.sr, 3200.0, (high - 0.5) * 30.0);
        self.last_high = high;
    }

    /// `dist` 0–1, `low` 0–1, `high` 0–1, `level` 0–1.
    #[inline]
    pub fn process(&mut self, sample: f32, dist: f32, low: f32, high: f32, level: f32) -> f32 {
        if param_changed(low, self.last_low) {
            self.update_low(low);
        }
        if param_changed(high, self.last_high) {
            self.update_high(high);
        }

        let x = self.dc_block.process(sample);
        let x = self.input_hp.process(x);
        let x = self.mid_scoop.process(x);

        // First-stage gain (1×–29×). A normal note (~0.3) still slams both stages
        // to the rail, so loud saturation and output level are unchanged — but the
        // far lower *small-signal* gain (was 56×) means the pedal no longer blows
        // up the hiss and hum riding under and between notes by ~10 dB. The old
        // ~86× cascade small-signal gain was the ML-2's "noise": a high-gain clip
        // amplifies a quiet noise floor by its small-signal slope, and only the
        // loud, rail-pinned peaks are unaffected by pulling that slope down.
        let gain = 1.0 + dist * 21.0;

        // 8× oversampled cascaded clip: tighten (pre-clip HP), saturate, then push
        // into a second, gentler stage. `pre_clip_hp` is borrowed directly so it
        // doesn't alias the `os` borrow.
        let pre_clip_hp = &mut self.pre_clip_hp;
        let pre_clip_lp = &mut self.pre_clip_lp;
        let x = self.os.process(x, |u| {
            let u = pre_clip_lp.process(pre_clip_hp.process(u));
            let s1 = ml2_stage(u * gain);
            // Second cascaded stage adds the long, compressed metal sustain. Its
            // push is kept modest (1.6×) so the cascade saturates smoothly instead
            // of squaring the wave into a fizzy buzz, and so it doesn't re-amplify
            // the noise floor. Output scaled to keep the pedal's level unchanged.
            ml2_stage(s1 * 1.5) * 0.9
        });

        // Tighten the low end the clipper produced (and strip the asymmetry DC).
        let x = self.post_clip_hp.process(x);
        // Active two-band EQ.
        let x = self.low_shelf.process(x);
        let x = self.high_shelf.process(x);
        // Tame the residual top-end fizz before the amp.
        let x = self.post_clip_lp.process(x);

        x * level * 0.5
    }
}

/// One ML-2 clip stage: a `tanh` soft-clip with a slight asymmetry (the negative
/// half saturates toward −1.08 instead of −1.0), so cascading two of them keeps a
/// touch of even-harmonic warmth rather than the purely-odd buzz of a symmetric
/// clipper. `tanh` already asymptotes to its limit, so no extra clamp is needed;
/// the small DC the asymmetry leaves is removed by the post-clip high-pass.
#[inline]
fn ml2_stage(x: f32) -> f32 {
    // The negative half saturates toward a higher rail (−1.3 vs +1.0). Asymmetric
    // *rail levels* (unlike a DC bias, which just shifts a slammed square and
    // washes out to DC the post-clip HP removes) survive the cascade's hard
    // saturation, so they inject a genuine, audible 2nd harmonic — the warmth that
    // keeps the voice from the clinical, purely-odd buzz of a symmetric clipper.
    // The DC the asymmetry leaves is removed downstream by the post-clip HP.
    if x >= 0.0 {
        x.tanh()
    } else {
        const T: f32 = 1.3;
        T * (x / T).tanh()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const SR: f32 = 48_000.0;

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

    fn rms(v: &[f32]) -> f64 {
        (v.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>() / v.len() as f64).sqrt()
    }

    /// Render a steady sine through the ML-2 and return the settled tail.
    fn render(f0: f32, amp: f32, dist: f32, low: f32, high: f32, level: f32) -> Vec<f32> {
        let mut m = MetalCore::new(SR);
        let n = SR as usize;
        let warmup = n / 4;
        let mut out = Vec::with_capacity(n - warmup);
        for i in 0..n {
            let x = (2.0 * PI * f0 * i as f32 / SR).sin() * amp;
            let y = m.process(x, dist, low, high, level);
            assert!(y.is_finite(), "ML-2 non-finite at {i}");
            if i >= warmup {
                out.push(y);
            }
        }
        out
    }

    /// The ML-2 must stay finite and bounded driving a hot low note, and its DIST
    /// knob must add saturation harmonics monotonically.
    #[test]
    fn finite_bounded_and_dist_adds_harmonics() {
        let mut max_abs = 0.0f32;
        for n in 0..(SR as usize) {
            let mut m = MetalCore::new(SR);
            let x = (2.0 * PI * 110.0 * n as f32 / SR).sin() * 0.7;
            let y = m.process(x, 0.8, 0.5, 0.5, 0.7);
            assert!(y.is_finite() && y.abs() < 2.0, "ML-2 unbounded: {y}");
            max_abs = max_abs.max(y.abs());
        }
        assert!(max_abs < 2.0, "ML-2 output out of bounds: {max_abs}");

        let thd = |dist: f32| {
            let out = render(330.0, 0.15, dist, 0.5, 0.5, 0.65);
            let fund = goertzel(&out, 330.0, SR);
            let upper: f32 = (2..=10).map(|k| goertzel(&out, 330.0 * k as f32, SR)).sum();
            upper / fund.max(1e-9)
        };
        let (lo, mid, hi) = (thd(0.05), thd(0.4), thd(0.9));
        assert!(
            hi > mid && mid > lo,
            "dist did not add harmonics monotonically: {lo:.3} -> {mid:.3} -> {hi:.3}"
        );
    }

    /// The active LOW knob must control the low end: boosting it raises low-frequency
    /// energy well above cutting it.
    #[test]
    fn low_knob_controls_lows() {
        let low_energy = |low: f32| goertzel(&render(100.0, 0.1, 0.05, low, 0.5, 0.65), 100.0, SR);
        assert!(
            low_energy(0.9) > low_energy(0.1) * 1.5,
            "LOW knob did not control the low band"
        );
    }

    /// The active HIGH knob must control the top end: boosting it raises
    /// high-frequency energy well above cutting it.
    #[test]
    fn high_knob_controls_highs() {
        let high_energy =
            |high: f32| goertzel(&render(4000.0, 0.1, 0.05, 0.5, high, 0.65), 4000.0, SR);
        assert!(
            high_energy(0.9) > high_energy(0.1) * 1.5,
            "HIGH knob did not control the high band"
        );
    }

    /// The LEVEL knob must scale the output.
    #[test]
    fn level_scales_output() {
        let out_lo = rms(&render(220.0, 0.2, 0.6, 0.5, 0.5, 0.3));
        let out_hi = rms(&render(220.0, 0.2, 0.6, 0.5, 0.5, 0.9));
        assert!(out_hi > out_lo * 1.5, "LEVEL knob did not scale output");
    }

    /// The cascaded asymmetric clipper must not leave a DC offset on a sustained low
    /// note — the post-clip high-pass removes the asymmetry's bias.
    #[test]
    fn output_has_no_dc_offset() {
        for f0 in [82.41f32, 220.0, 440.0] {
            let out = render(f0, 0.5, 0.7, 0.5, 0.5, 0.65);
            let mean = out.iter().map(|&x| x as f64).sum::<f64>() / out.len() as f64;
            assert!(mean.abs() < 1e-3, "{f0} Hz: DC offset {mean:.6}");
        }
    }

    /// The ML-2 is an odd-harmonic pedal, but the asymmetric rail levels must add a
    /// *measurable* dose of even-harmonic warmth so the voice isn't the clinical,
    /// purely-odd buzz a symmetric clipper makes. Odd content still clearly
    /// dominates (keeps the metal character); a real even component is present (the
    /// warmth). Guards against the asymmetry washing back out to nothing.
    #[test]
    fn clip_has_even_harmonic_warmth_but_stays_odd_dominant() {
        for (f0, dist) in [(196.0f32, 0.5f32), (330.0, 0.7)] {
            let out = render(f0, 0.3, dist, 0.5, 0.5, 0.65);
            let h1 = goertzel(&out, f0, SR);
            let even: f32 = (2..=8)
                .step_by(2)
                .map(|k| goertzel(&out, f0 * k as f32, SR))
                .sum();
            let odd: f32 = (3..=9)
                .step_by(2)
                .map(|k| goertzel(&out, f0 * k as f32, SR))
                .sum();
            assert!(
                odd > even * 3.0,
                "{f0} Hz: not odd-dominant (odd {odd:.4}, even {even:.4})"
            );
            assert!(
                even > h1 * 0.01,
                "{f0} Hz: no even-harmonic warmth (even/h1 {:.4}) — asymmetry washed out?",
                even / h1
            );
        }
    }

    /// The ML-2's small-signal gain must stay in check: a real guitar carries a
    /// quiet bed of hiss and hum under the notes, and an over-hot cascade blows it
    /// up into the wash of "noise" the ML-2 is infamous for. Fed a −50 dBFS
    /// noise+hum bed, the output must stay well below the runaway level the old
    /// ~86× cascade produced (it made "silence" hotter than −40 dBFS). Loud notes
    /// are pinned to the clip rail regardless, so this costs no saturation.
    #[test]
    fn noise_floor_stays_low() {
        let mut m = MetalCore::new(SR);
        let mut seed = 0x9E3779B9u32;
        let warm = (SR * 0.1) as usize;
        let n = (SR * 0.6) as usize;
        let mut sum = 0.0f64;
        let mut count = 0u32;
        for i in 0..n {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let white = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
            let t = i as f32 / SR;
            let hum = 0.0015 * (2.0 * PI * 60.0 * t).sin();
            let y = m.process(0.003 * white + hum, 0.7, 0.5, 0.5, 0.65);
            if i >= warm {
                sum += (y * y) as f64;
                count += 1;
            }
        }
        let out_db = 20.0 * (sum / count as f64).sqrt().max(1e-12).log10();
        assert!(
            out_db < -42.0,
            "ML-2 noise floor too hot ({out_db:.1} dBFS) — small-signal gain runaway?"
        );
    }

    /// 8× oversampling must keep the aggressive clip harmonic, not aliased — energy
    /// stays on the exact harmonics rather than scattering into inharmonic fizz.
    #[test]
    fn output_is_harmonic_not_aliased() {
        for (f0, dist) in [(220.0f32, 0.6f32), (330.0, 0.8), (440.0, 0.95)] {
            let out = render(f0, 0.3, dist, 0.5, 0.5, 0.65);
            let harm_pow: f64 = (1..=12)
                .map(|k| (goertzel(&out, f0 * k as f32, SR) as f64).powi(2) / 2.0)
                .sum();
            let frac = harm_pow / rms(&out).powi(2);
            assert!(
                frac > 0.9,
                "{f0} Hz: only {:.1}% of energy is harmonic — aliasing present",
                100.0 * frac
            );
        }
    }
}
