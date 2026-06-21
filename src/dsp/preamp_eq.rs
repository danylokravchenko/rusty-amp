use crate::dsp::biquad::Biquad;

/// Pre-amp EQ — sits *before* the amp, so it shapes what the gain stage actually
/// clips (an MXR/GE-7-style "secret weapon": scoop the mids going in for a tighter
/// chug, or push them for lead sustain). This is a different job from the post-cab
/// [`ParametricEq`](super::parametric_eq::ParametricEq), which colours the final
/// mix after distortion. Mono (the pre-amp path is mono). Knobs 0–1 map to ±12 dB,
/// centre (0.5) = flat.
pub struct PreampEq {
    sr: f32,
    low: Biquad,
    mid: Biquad,
    high: Biquad,
    last_low: f32,
    last_mid: f32,
    last_high: f32,
}

impl PreampEq {
    pub fn new(sr: f32) -> Self {
        let mut eq = Self {
            sr,
            low: Biquad::low_shelf(sr, 100.0, 0.0),
            mid: Biquad::peak_eq(sr, 650.0, 1.0, 0.0),
            high: Biquad::high_shelf(sr, 3000.0, 0.0),
            last_low: -1.0,
            last_mid: -1.0,
            last_high: -1.0,
        };
        eq.rebuild(0.5, 0.5, 0.5);
        eq
    }

    fn rebuild(&mut self, low: f32, mid: f32, high: f32) {
        let db = |v: f32| (v - 0.5) * 24.0; // 0 → −12 dB, 0.5 → 0, 1 → +12 dB
        self.low = Biquad::low_shelf(self.sr, 100.0, db(low));
        self.mid = Biquad::peak_eq(self.sr, 650.0, 1.0, db(mid));
        self.high = Biquad::high_shelf(self.sr, 3000.0, db(high));
        self.last_low = low;
        self.last_mid = mid;
        self.last_high = high;
    }

    #[inline]
    pub fn process(&mut self, x: f32, low: f32, mid: f32, high: f32) -> f32 {
        if (low - self.last_low).abs() > 0.001
            || (mid - self.last_mid).abs() > 0.001
            || (high - self.last_high).abs() > 0.001
        {
            self.rebuild(low, mid, high);
        }
        self.high.process(self.mid.process(self.low.process(x)))
    }
}
