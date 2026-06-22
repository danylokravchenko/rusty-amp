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
///   • **Symmetric** clipping (two anti-parallel silicon diodes to ground) — no DC
///     offset, and a tight cubic knee that preserves note definition.
///   • The tone control is a **tilt** (bass↔treble seesaw around ~1 kHz), like the
///     real pedal — NOT a mid scoop. The old LP/HP-blend tone scooped the mids,
///     which is exactly what left the low end loose and the top fizzy.
///   • 4× oversampling keeps the clip harmonics above the audible band; the
///     post-clip HP and the downstream cab low-pass mop up the rest.
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

        x * level * 0.6
    }
}

/// DS-1 diode clipper: symmetric silicon clipping with a tight cubic knee.
///
/// Two anti-parallel diodes to ground clip both half-cycles identically
/// (symmetric → no DC offset). The cubic soft-clip up to ±1 then hard limit gives
/// a tight, defined knee — far less of the compressed "mush" that a slow
/// asymptotic knee produces, which keeps low notes articulate instead of loose.
#[inline]
fn ds1_clip(x: f32) -> f32 {
    if x <= -1.0 {
        -1.0
    } else if x >= 1.0 {
        1.0
    } else {
        1.5 * x - 0.5 * x * x * x
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// Single-bin magnitude estimate via the Goertzel algorithm.
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

    /// The DS-1's symmetric clipper must not inject a DC offset on a sustained low
    /// note — a wandering DC bias was the source of the "farty" blocking distortion.
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
}
