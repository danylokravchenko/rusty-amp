use super::{ThreeBandEq, param_changed};

// Band layout for the post-cab parametric EQ.
const LOW_FREQ: f32 = 120.0;
const MID_FREQ: f32 = 800.0;
const MID_Q: f32 = 1.5;
const HIGH_FREQ: f32 = 5000.0;

/// 3-band parametric EQ: low shelf, mid peak, high shelf.
/// All gain knobs are normalised 0–1, mapping to ±15 dB.
/// Stereo: independent filter state per channel ([`ThreeBandEq`] each), same coefficients.
pub struct ParametricEq {
    left: ThreeBandEq,
    right: ThreeBandEq,
    last_low: f32,
    last_mid: f32,
    last_high: f32,
}

impl ParametricEq {
    pub fn new(sr: f32) -> Self {
        let mut eq = Self {
            left: ThreeBandEq::new(sr, LOW_FREQ, MID_FREQ, MID_Q, HIGH_FREQ),
            right: ThreeBandEq::new(sr, LOW_FREQ, MID_FREQ, MID_Q, HIGH_FREQ),
            last_low: -1.0,
            last_mid: -1.0,
            last_high: -1.0,
        };
        eq.rebuild(0.5, 0.5, 0.5);
        eq
    }

    fn rebuild(&mut self, low: f32, mid: f32, high: f32) {
        let db = |v: f32| (v - 0.5) * 30.0; // 0→-15 dB, 0.5→0, 1→+15 dB
        self.left.set_gains_db(db(low), db(mid), db(high));
        self.right.set_gains_db(db(low), db(mid), db(high));
        self.last_low = low;
        self.last_mid = mid;
        self.last_high = high;
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32, low: f32, mid: f32, high: f32) -> (f32, f32) {
        if param_changed(low, self.last_low)
            || param_changed(mid, self.last_mid)
            || param_changed(high, self.last_high)
        {
            self.rebuild(low, mid, high);
        }
        (self.left.process(l), self.right.process(r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// Boosting a band must raise that band's level; both channels stay finite.
    #[test]
    fn bands_boost_and_cut_and_are_finite() {
        let sr = 48_000.0;
        // RMS of the left channel at `freq` for the given (low, mid, high) knobs.
        let rms = |knobs: (f32, f32, f32), freq: f32| {
            let mut eq = ParametricEq::new(sr);
            let mut sum = 0.0f64;
            for n in 0..(sr as usize) {
                let x = (2.0 * PI * freq * n as f32 / sr).sin();
                let (l, r) = eq.process(x, x, knobs.0, knobs.1, knobs.2);
                assert!(l.is_finite() && r.is_finite());
                if n >= sr as usize / 2 {
                    sum += (l * l) as f64;
                }
            }
            sum.sqrt()
        };

        assert!(
            rms((1.0, 0.5, 0.5), 60.0) > rms((0.0, 0.5, 0.5), 60.0),
            "low band dead"
        );
        assert!(
            rms((0.5, 1.0, 0.5), 800.0) > rms((0.5, 0.0, 0.5), 800.0),
            "mid band dead"
        );
        assert!(
            rms((0.5, 0.5, 1.0), 10_000.0) > rms((0.5, 0.5, 0.0), 10_000.0),
            "high band dead"
        );
    }
}
