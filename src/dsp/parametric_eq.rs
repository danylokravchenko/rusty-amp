use crate::dsp::biquad::Biquad;

/// 3-band parametric EQ: low shelf, mid peak, high shelf.
/// All gain knobs are normalised 0–1, mapping to ±15 dB.
pub struct ParametricEq {
    sr: f32,
    low: Biquad,
    mid: Biquad,
    high: Biquad,
    last_low: f32,
    last_mid: f32,
    last_high: f32,
}

impl ParametricEq {
    pub fn new(sr: f32) -> Self {
        let mut eq = Self {
            sr,
            low: Biquad::low_shelf(sr, 120.0, 0.0),
            mid: Biquad::peak_eq(sr, 800.0, 1.5, 0.0),
            high: Biquad::high_shelf(sr, 5000.0, 0.0),
            last_low: -1.0,
            last_mid: -1.0,
            last_high: -1.0,
        };
        eq.rebuild(0.5, 0.5, 0.5);
        eq
    }

    fn rebuild(&mut self, low: f32, mid: f32, high: f32) {
        let db = |v: f32| (v - 0.5) * 30.0; // 0→-15 dB, 0.5→0, 1→+15 dB
        self.low = Biquad::low_shelf(self.sr, 120.0, db(low));
        self.mid = Biquad::peak_eq(self.sr, 800.0, 1.5, db(mid));
        self.high = Biquad::high_shelf(self.sr, 5000.0, db(high));
        self.last_low = low;
        self.last_mid = mid;
        self.last_high = high;
    }

    #[inline]
    pub fn process(&mut self, sample: f32, low: f32, mid: f32, high: f32) -> f32 {
        if (low - self.last_low).abs() > 0.001
            || (mid - self.last_mid).abs() > 0.001
            || (high - self.last_high).abs() > 0.001
        {
            self.rebuild(low, mid, high);
        }
        let x = self.low.process(sample);
        let x = self.mid.process(x);
        self.high.process(x)
    }
}
