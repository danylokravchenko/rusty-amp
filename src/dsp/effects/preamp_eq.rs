use super::{ThreeBandEq, param_changed};

// Band layout for the pre-amp EQ — lower mid centre and gentler Q than the
// post-cab parametric EQ, voiced to shape what the gain stage clips.
const LOW_FREQ: f32 = 100.0;
const MID_FREQ: f32 = 650.0;
const MID_Q: f32 = 1.0;
const HIGH_FREQ: f32 = 3000.0;

/// Pre-amp EQ — sits *before* the amp, so it shapes what the gain stage actually
/// clips (an MXR/GE-7-style "secret weapon": scoop the mids going in for a tighter
/// chug, or push them for lead sustain). This is a different job from the post-cab
/// [`ParametricEq`](super::parametric_eq::ParametricEq), which colours the final
/// mix after distortion. Mono (the pre-amp path is mono). Knobs 0–1 map to ±12 dB,
/// centre (0.5) = flat.
pub struct PreampEq {
    eq: ThreeBandEq,
    last_low: f32,
    last_mid: f32,
    last_high: f32,
}

impl PreampEq {
    pub fn new(sr: f32) -> Self {
        let mut eq = Self {
            eq: ThreeBandEq::new(sr, LOW_FREQ, MID_FREQ, MID_Q, HIGH_FREQ),
            last_low: -1.0,
            last_mid: -1.0,
            last_high: -1.0,
        };
        eq.rebuild(0.5, 0.5, 0.5);
        eq
    }

    fn rebuild(&mut self, low: f32, mid: f32, high: f32) {
        let db = |v: f32| (v - 0.5) * 24.0; // 0 → −12 dB, 0.5 → 0, 1 → +12 dB
        self.eq.set_gains_db(db(low), db(mid), db(high));
        self.last_low = low;
        self.last_mid = mid;
        self.last_high = high;
    }

    #[inline]
    pub fn process(&mut self, x: f32, low: f32, mid: f32, high: f32) -> f32 {
        if param_changed(low, self.last_low)
            || param_changed(mid, self.last_mid)
            || param_changed(high, self.last_high)
        {
            self.rebuild(low, mid, high);
        }
        self.eq.process(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// Each knob must move its band and the output must stay finite. Centre (0.5)
    /// is unity, so a boost raises and a cut lowers the band's level.
    #[test]
    fn bands_track_their_knobs() {
        let sr = 48_000.0;
        let rms = |knobs: (f32, f32, f32), freq: f32| {
            let mut eq = PreampEq::new(sr);
            let mut sum = 0.0f64;
            for n in 0..(sr as usize) {
                let x = (2.0 * PI * freq * n as f32 / sr).sin();
                let y = eq.process(x, knobs.0, knobs.1, knobs.2);
                assert!(y.is_finite());
                if n >= sr as usize / 2 {
                    sum += (y * y) as f64;
                }
            }
            sum.sqrt()
        };

        assert!(
            rms((1.0, 0.5, 0.5), 60.0) > rms((0.0, 0.5, 0.5), 60.0),
            "low band dead"
        );
        assert!(
            rms((0.5, 1.0, 0.5), 650.0) > rms((0.5, 0.0, 0.5), 650.0),
            "mid band dead"
        );
        assert!(
            rms((0.5, 0.5, 1.0), 8000.0) > rms((0.5, 0.5, 0.0), 8000.0),
            "high band dead"
        );
    }
}
