use super::biquad::Biquad;
use crate::dsp::oversample::Oversampler8;

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
///   • 8× oversampling keeps the clip harmonics above the audible band.
pub struct Distortion {
    sr: f32,
    dc_block: Biquad,
    input_hp: Biquad,
    // Mid-focused pre-clip emphasis (base rate) — the DS-1's voice + definition.
    mid_emphasis: Biquad,
    os: Oversampler8,
    // Pre-clip HP at 8× rate — tightens the low end before the clipper.
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
        let sr8 = sr * 8.0;
        let mut d = Self {
            sr,
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            input_hp: Biquad::highpass(sr, 80.0, 0.707),
            // +3 dB around 800 Hz: gentle mid focus for definition (kept small so
            // it doesn't pump the output level into the amp).
            mid_emphasis: Biquad::peak_eq(sr, 800.0, 0.9, 3.0),
            os: Oversampler8::new(sr),
            // 130 Hz pre-clip HP: trims low-mid mud before the clipper.
            pre_clip_hp: Biquad::highpass(sr8, 130.0, 0.707),
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
        if (tone - self.last_tone).abs() > 0.001 {
            self.update_tone(tone);
        }

        let x = self.dc_block.process(sample);
        let x = self.input_hp.process(x);
        let x = self.mid_emphasis.process(x);

        let gain = 1.0 + drive * 60.0;

        // ── 8× oversampled clip stage ─────────────────────────────────────────
        let up = self.os.upsample(x);
        let mut down = [0.0f32; 8];
        for (o, &u) in down.iter_mut().zip(up.iter()) {
            let u = self.pre_clip_hp.process(u); // tighten lows before clipping
            *o = ds1_clip(u * gain) / gain.sqrt();
        }
        let x = self.os.downsample(down);
        // ── end oversampled section ───────────────────────────────────────────

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
