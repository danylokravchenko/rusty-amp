use super::param_changed;
use crate::dsp::biquad::Biquad;
use crate::dsp::oversample::Oversampler4;

/// Boss DS-1 Distortion simulation.
///
/// Signal path:
///   DC block → input HP → mid-emphasis → [4× OS: pre-clip HP → symmetric clip] → tilt tone → level
///
/// DS-1 character & authenticity:
///   • The real DS-1 is a **thin, tight, mid-forward** pedal — it is not a
///     bass-heavy fuzz. A blubbery low end means too much low-frequency energy is
///     getting clipped and dumped into the amp. We tighten it on both sides of the
///     clipper: a pre-clip high-pass removes low-mid mud before clipping, and a
///     post-clip high-pass removes the woof the clipper generates so the output
///     stays articulate going into the amp.
///   • A small mid-emphasis before the clipper gives the DS-1 its characteristic
///     mid honk and note definition (without pumping the output level).
///   • **Near-symmetric** clipping with a tight cubic knee that preserves note
///     definition. A perfectly symmetric clipper produces *only odd harmonics* —
///     the buzzy, clinical, "electronic" sound. The real DS-1's op-amp clipping
///     stage is slightly asymmetric, so we clip the negative half a hair later than
///     the positive half. That adds a touch of **2nd-harmonic warmth** (the
///     difference between "production-grade" and "fizzy solid-state") while staying
///     tight; the small DC the asymmetry creates is removed by the 150 Hz post-clip
///     high-pass, so there is still no DC offset to fart out.
///   • The tone control is a **tilt** (bass↔treble seesaw around ~1 kHz), like the
///     real pedal — NOT a mid scoop. The old LP/HP-blend tone scooped the mids,
///     which is exactly what left the low end loose and the top fizzy.
///   • 4× oversampling keeps the clip harmonics above the audible band; a gentle
///     post-clip low-pass tames the residual top-end fizz *before* it reaches the
///     amp's gain stage (where it would otherwise intermodulate into harshness),
///     and the post-clip HP plus the downstream cab low-pass mop up the rest.
pub struct Distortion {
    sr: f32,
    dc_block: Biquad,
    input_hp: Biquad,
    // Mid-focused pre-clip emphasis (base rate) — the DS-1's voice + definition.
    mid_emphasis: Biquad,
    os: Oversampler4,
    // Pre-clip HP at 4× rate — tightens the low end before the clipper.
    pre_clip_hp: Biquad,
    // Post-clip HP (base rate) — removes the blubber the clipper generates so the
    // DS-1 doesn't dump a woofy low end into the amp.
    post_clip_hp: Biquad,
    // Post-clip LP (base rate) — tames the harsh clip fizz above the guitar's
    // useful range so it doesn't intermodulate in the amp's gain stage.
    post_clip_lp: Biquad,
    // Tilt tone control (base rate): low + high shelves driven in opposition.
    tone_low: Biquad,
    tone_high: Biquad,
    last_tone: f32,
}

impl Distortion {
    pub fn new(sr: f32) -> Self {
        let sr4 = sr * 4.0;
        let mut d = Self {
            sr,
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            input_hp: Biquad::highpass(sr, 80.0, 0.707),
            // +3 dB around 800 Hz: gentle mid focus for definition (kept small so
            // it doesn't pump the output level into the amp).
            mid_emphasis: Biquad::peak_eq(sr, 800.0, 0.9, 3.0),
            os: Oversampler4::new(sr),
            // 130 Hz pre-clip HP: trims low-mid mud before the clipper.
            pre_clip_hp: Biquad::highpass(sr4, 130.0, 0.707),
            // 150 Hz post-clip HP: the decisive tightener — keeps the low E present
            // but strips the loose, blubbery woof the clipper produces.
            post_clip_hp: Biquad::highpass(sr, 150.0, 0.707),
            // 6.5 kHz post-clip LP: keeps the DS-1's bite and articulation but
            // rolls off the brittle clip fizz above it, like the real pedal's
            // output stage. Gentle (0.707) so it darkens nothing in the body.
            post_clip_lp: Biquad::lowpass(sr, 6500.0, 0.707),
            tone_low: Biquad::low_shelf(sr, 500.0, 0.0),
            tone_high: Biquad::high_shelf(sr, 1800.0, 0.0),
            last_tone: -1.0,
        };
        d.update_tone(0.5);
        d
    }

    fn update_tone(&mut self, tone: f32) {
        // Tilt: tone up → cut bass / boost treble; tone down → boost bass / cut
        // treble. Centre (0.5) is flat. ±12 dB seesaw around the ~1 kHz pivot.
        let tilt = (tone - 0.5) * 24.0;
        self.tone_low = Biquad::low_shelf(self.sr, 500.0, -tilt);
        self.tone_high = Biquad::high_shelf(self.sr, 1800.0, tilt);
        self.last_tone = tone;
    }

    #[inline]
    pub fn process(&mut self, sample: f32, drive: f32, tone: f32, level: f32) -> f32 {
        if param_changed(tone, self.last_tone) {
            self.update_tone(tone);
        }

        let x = self.dc_block.process(sample);
        let x = self.input_hp.process(x);
        let x = self.mid_emphasis.process(x);

        let gain = 1.0 + drive * 60.0;

        // 4× oversampled clip stage — tighten lows (pre-clip HP) then clip, per
        // high-rate sample. `pre_clip_hp` is borrowed directly so it doesn't alias
        // the `os` borrow inside the closure.
        let pre_clip_hp = &mut self.pre_clip_hp;
        let x = self.os.process(x, |u| {
            let u = pre_clip_hp.process(u);
            ds1_clip(u * gain) / gain.sqrt()
        });

        // Tighten the low end the clipper produced, then apply the tilt tone.
        let x = self.post_clip_hp.process(x);
        let x = self.tone_low.process(x);
        let x = self.tone_high.process(x);
        // Tame the residual top-end clip fizz before it hits the amp.
        let x = self.post_clip_lp.process(x);

        x * level * 0.6
    }
}

/// DS-1 diode clipper: near-symmetric silicon clipping with a tight cubic knee.
///
/// Each half-cycle is shaped by a cubic soft-clip up to its threshold, then hard
/// limited: `t·(1.5u − 0.5u³)` with `u = x/t`. That curve has unit-ish small-signal
/// gain (slope 1.5 at the origin) and reaches the limit `t` with *zero* slope, so
/// it joins the flat top with no derivative kink — a tight, defined knee instead of
/// the compressed "mush" a slow asymptotic knee produces, which keeps low notes
/// articulate instead of loose.
///
/// The negative half clips a hair later than the positive half (threshold 1.12 vs
/// 1.0), mirroring the DS-1 op-amp clipping stage's slight asymmetry. A perfectly
/// symmetric clipper makes only odd harmonics — the buzzy, electronic sound; the
/// asymmetry adds the warm 2nd harmonic. The tiny resulting DC bias is removed
/// downstream by the 150 Hz post-clip high-pass, so there is still no DC to fart.
#[inline]
fn ds1_clip(x: f32) -> f32 {
    /// Cubic soft-clip of a non-negative value `v` toward threshold `t`.
    #[inline]
    fn knee(v: f32, t: f32) -> f32 {
        if v >= t {
            t
        } else {
            let u = v / t;
            t * (1.5 * u - 0.5 * u * u * u)
        }
    }
    if x >= 0.0 {
        knee(x, 1.0)
    } else {
        -knee(-x, 1.12)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const SR: f32 = 48_000.0;

    /// Single-bin amplitude estimate via the Goertzel algorithm (a unit sine reads
    /// ~1.0 over integer cycles). Computed in `f64`: the recurrence's poles sit on
    /// the unit circle, so over these long windows an `f32` accumulator loses the
    /// small bins (e.g. a recovered fundamental can read as noise).
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

    /// Render a steady sine through the DS-1 and return the settled tail. `f0` should
    /// divide the window into whole cycles so the Goertzel bins land on harmonics.
    fn render(f0: f32, amp: f32, drive: f32, tone: f32, level: f32) -> Vec<f32> {
        let mut ds = Distortion::new(SR);
        let n = SR as usize;
        let warmup = n / 4;
        let mut out = Vec::with_capacity(n - warmup);
        for i in 0..n {
            let x = (2.0 * PI * f0 * i as f32 / SR).sin() * amp;
            let y = ds.process(x, drive, tone, level);
            assert!(y.is_finite(), "DS-1 non-finite at {i}");
            if i >= warmup {
                out.push(y);
            }
        }
        out
    }

    /// The DS-1's near-symmetric clipper must not inject a DC offset on a sustained
    /// low note — a wandering DC bias was the source of the "farty" blocking
    /// distortion, and the slight diode asymmetry's DC is removed by the post-clip HP.
    #[test]
    fn no_dc_offset_on_low_e() {
        let sr = 48_000.0;
        let mut ds = Distortion::new(sr);
        let mut sum = 0.0f64;
        let mut count = 0u32;
        let warmup = sr as usize / 4; // let filters settle
        for n in 0..(sr as usize) {
            let x = (2.0 * PI * 82.41 * n as f32 / sr).sin() * 0.7;
            let y = ds.process(x, 0.8, 0.5, 0.7);
            if n >= warmup {
                sum += y as f64;
                count += 1;
            }
        }
        let dc = (sum / count as f64).abs();
        assert!(dc < 0.01, "distortion has DC offset (fart risk): {dc}");
    }

    /// The DS-1 must stay tight, not blubbery: the "woof" energy at/below the low-E
    /// fundamental should be a small fraction of the body harmonics, and the pedal
    /// must not pump a hot level into the amp. Guards against regressing to the
    /// loose, bass-heavy voicing.
    #[test]
    fn low_end_balance() {
        let sr = 48_000.0;
        let mut ds = Distortion::new(sr);
        let e2 = 82.41;
        let n = sr as usize;
        let warmup = sr as usize / 4;
        let mut out = Vec::with_capacity(n - warmup);
        let mut in_buf = Vec::with_capacity(n - warmup);
        for i in 0..n {
            let t = i as f32 / sr;
            let x = 0.15
                * ((2.0 * PI * e2 * t).sin()
                    + 0.5 * (2.0 * PI * 2.0 * e2 * t).sin()
                    + 0.3 * (2.0 * PI * 3.0 * e2 * t).sin());
            let y = ds.process(x, 0.5, 0.5, 0.65);
            if i >= warmup {
                out.push(y);
                in_buf.push(x);
            }
        }
        let rms =
            |v: &[f32]| (v.iter().map(|s| (s * s) as f64).sum::<f64>() / v.len() as f64).sqrt();
        let m = |v: &[f32], f| goertzel(v, f, sr);
        let woof = m(&out, 55.0) + m(&out, 82.41) + m(&out, 110.0);
        let body = m(&out, 165.0) + m(&out, 247.0) + m(&out, 330.0);
        let through = rms(&out) / rms(&in_buf);
        let ratio = woof / body.max(1e-9);
        assert!(
            ratio < 0.5,
            "DS-1 low end is blubbery: woof/body = {ratio:.2}"
        );
        assert!(
            through < 1.0,
            "DS-1 output too hot, will slam the amp: {through:.2}x"
        );
    }

    // ── Clip transfer (unit tests on the diode model itself) ──────────────────

    /// The cubic diode clipper must be a bipolar saturator: sign-preserving, bounded
    /// by its two thresholds (+1.0 / −1.12), monotonic, and *smooth* (no derivative
    /// kink where the cubic knee meets the flat top — a kink injects buzzy high
    /// harmonics). The deeper negative threshold is the deliberate asymmetry.
    #[test]
    fn clip_is_bounded_sign_preserving_monotonic_and_smooth() {
        let mut prev = f32::NEG_INFINITY;
        let mut last_y = ds1_clip(-4.0);
        let mut max_step = 0.0f32;
        let mut x = -4.0f32;
        while x <= 4.0 {
            let y = ds1_clip(x);
            assert!(y.is_finite(), "clip non-finite at {x}");
            assert!(
                (-1.1201..=1.0001).contains(&y),
                "clip out of bounds at {x}: {y}"
            );
            if x > 0.01 {
                assert!(
                    y > 0.0,
                    "positive input {x} produced non-positive {y} (rectifying!)"
                );
            }
            if x < -0.01 {
                assert!(
                    y < 0.0,
                    "negative input {x} produced non-negative {y} (rectifying!)"
                );
            }
            assert!(y >= prev - 1e-6, "clip not monotonic at {x}: {y} < {prev}");
            max_step = max_step.max((y - last_y).abs());
            last_y = y;
            prev = y;
            x += 0.005;
        }
        // Small-signal slope is ~1.5, so the step over dx=0.005 is ~0.0075; a hard
        // corner at the knee would blow well past this.
        assert!(
            max_step < 0.02,
            "clip transfer has a kink (max step {max_step:.4})"
        );
        // Asymmetry (warmth) and the two diode thresholds.
        assert!(
            ds1_clip(-0.5).abs() > ds1_clip(0.5).abs(),
            "clip not asymmetric"
        );
        assert!((ds1_clip(5.0) - 1.0).abs() < 1e-4, "positive threshold off");
        assert!(
            (ds1_clip(-5.0) + 1.12).abs() < 1e-4,
            "negative threshold off"
        );
    }

    // ── Full-stage spectral behaviour ─────────────────────────────────────────

    /// The DS-1 is an aggressive, *odd-harmonic* pedal — that is its voice. But a
    /// perfectly symmetric clipper makes only odd harmonics and sounds clinical and
    /// buzzy; the slight diode asymmetry adds a small, deliberate dose of even
    /// harmonics for warmth. We require odd content to clearly dominate (keeps the
    /// DS-1 character) while a measurable even component is present (the warmth that
    /// keeps it from sounding artificial).
    #[test]
    fn clip_is_odd_dominant_with_a_touch_of_even_warmth() {
        for (f0, d) in [(220.0f32, 0.4f32), (440.0, 0.6)] {
            let out = render(f0, 0.3, d, 0.5, 0.65);
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
                even > h1 * 0.005,
                "{f0} Hz: no even-harmonic warmth (even/h1 {:.4}) — clipper went symmetric?",
                even / h1
            );
        }
    }

    /// The slight diode asymmetry must not leave a DC offset on the output: the
    /// 150 Hz post-clip high-pass removes it. (Sustained low note → DC = "fart".)
    #[test]
    fn output_has_no_dc_offset_across_the_neck() {
        for f0 in [82.41f32, 220.0, 440.0] {
            let out = render(f0, 0.5, 0.7, 0.5, 0.65);
            let mean = out.iter().map(|&x| x as f64).sum::<f64>() / out.len() as f64;
            assert!(mean.abs() < 1e-3, "{f0} Hz: DC offset {mean:.6}");
        }
    }

    /// A properly oversampled clipper concentrates its energy on the exact harmonics
    /// of the input; aliasing (the digital "fizz/grit") would scatter energy onto
    /// inharmonic bins and drop this fraction.
    #[test]
    fn output_is_harmonic_not_aliased() {
        for (f0, drive) in [(220.0f32, 0.5f32), (440.0, 0.8), (660.0, 0.95)] {
            let out = render(f0, 0.3, drive, 0.5, 0.65);
            let harm_pow: f64 = (1..=12)
                .map(|k| (goertzel(&out, f0 * k as f32, SR) as f64).powi(2) / 2.0)
                .sum();
            let frac = harm_pow / rms(&out).powi(2);
            assert!(
                frac > 0.95,
                "{f0} Hz: only {:.1}% of energy is harmonic — aliasing present",
                100.0 * frac
            );
        }
    }

    /// Even at high drive the played note must remain the loudest partial, so the
    /// pitch stays defined instead of dissolving into buzz.
    #[test]
    fn fundamental_leads_the_spectrum() {
        for (f0, d) in [(220.0f32, 0.5f32), (330.0, 0.6), (440.0, 0.6)] {
            let out = render(f0, 0.3, d, 0.5, 0.65);
            let h: Vec<f32> = (1..=10)
                .map(|k| goertzel(&out, f0 * k as f32, SR))
                .collect();
            let peak = h.iter().cloned().fold(0.0f32, f32::max);
            assert!(
                h[0] >= peak * 0.99,
                "{f0} Hz: fundamental ({:.4}) is not the loudest partial ({peak:.4})",
                h[0]
            );
        }
    }

    /// Turning the drive up must add saturation harmonics monotonically.
    #[test]
    fn drive_increases_harmonic_distortion() {
        let thd = |drive: f32| {
            let out = render(330.0, 0.15, drive, 0.5, 0.65);
            let fund = goertzel(&out, 330.0, SR);
            let upper: f32 = (2..=10).map(|k| goertzel(&out, 330.0 * k as f32, SR)).sum();
            upper / fund.max(1e-9)
        };
        let (lo, mid, hi) = (thd(0.05), thd(0.4), thd(0.9));
        assert!(
            hi > mid && mid > lo,
            "drive did not add harmonics monotonically: {lo:.3} -> {mid:.3} -> {hi:.3}"
        );
    }

    /// The post-clip low-pass must tame the brittle top end before it reaches the
    /// amp: the near-linear through-gain falls monotonically from the mid band up,
    /// so the harsh clip fizz doesn't intermodulate downstream.
    #[test]
    fn post_clip_lp_tames_the_top_end() {
        let through = |f: f32| goertzel(&render(f, 0.05, 0.05, 0.5, 0.65), f, SR);
        let (mid, high, top) = (through(1000.0), through(6000.0), through(10_000.0));
        assert!(mid > high, "no roll-off by 6 kHz ({mid:.5} -> {high:.5})");
        assert!(high > top, "no roll-off by 10 kHz ({high:.5} -> {top:.5})");
        assert!(
            top < mid * 0.5,
            "top end not tamed: 10 kHz is {:.2}x the 1 kHz level",
            top / mid
        );
    }

    /// The tone control is a tilt: turning it up cuts bass and boosts treble, like
    /// the real pedal (not a mid scoop). Guards the seesaw shelves.
    #[test]
    fn tilt_tone_trades_bass_for_treble() {
        let band = |tone: f32, f: f32| goertzel(&render(f, 0.05, 0.05, tone, 0.65), f, SR);
        assert!(
            band(0.1, 100.0) > band(0.9, 100.0),
            "tone up did not cut bass"
        );
        assert!(
            band(0.9, 3000.0) > band(0.1, 3000.0),
            "tone up did not boost treble"
        );
    }
}
