use super::{db_to_lin, param_changed};
use crate::dsp::biquad::Biquad;

/// Boss GE-7 style graphic equalizer — seven fixed-frequency peak bands plus an
/// output level slider, run in stereo (post-cab).
///
/// A *graphic* EQ differs from the [`ParametricEq`](super::parametric_eq::ParametricEq)
/// and [`PreampEq`](super::preamp_eq::PreampEq) three-band shelves: instead of a
/// few movable shelves it exposes a fixed bank of narrow peak filters spaced ~1
/// octave apart, so the fader positions *draw* the frequency response — hence the
/// name. The GE-7's seven ISO-ish centres (100 Hz → 10 kHz) let you carve a
/// surgical mid scoop or a presence bump the broad shelves can't, and the output
/// level makes up (or trims) the gain the boosts add. Sits in the post-cab rack,
/// so it colours the finished, mic'd tone the way an EQ patched into the amp's
/// effects loop would.
///
/// Each band fader is 0–1 → ±12 dB (0.5 = flat); the level fader is 0–1 → ±15 dB
/// (0.5 = unity). Stereo: an identical filter bank per channel so it preserves the
/// cab's L/R decorrelation.
pub struct GraphicEq {
    sr: f32,
    // One peak biquad per band, per channel. The two channels share the same
    // gains — the level bank just runs on both sides of the stereo signal.
    left: [Biquad; BANDS],
    right: [Biquad; BANDS],
    last_bands: [f32; BANDS],
    last_level: f32,
    level_lin: f32,
}

/// Number of frequency bands (Boss GE-7: seven sliders).
pub const BANDS: usize = 7;

/// Band centre frequencies, low → high. Roughly one octave apart, matching the
/// GE-7's 100 Hz / 220 / 470 / 1k / 2.2k / 4.7k / 10k voicing.
const FREQS: [f32; BANDS] = [100.0, 220.0, 470.0, 1000.0, 2200.0, 4700.0, 10_000.0];

/// Constant Q for every band — narrow enough that each fader owns its octave
/// without smearing into its neighbours, wide enough that adjacent boosts still
/// sum smoothly into a broad curve.
const Q: f32 = 2.0;

/// A flat fader (0.5) is 0 dB; the ends are ±12 dB.
#[inline]
fn band_db(v: f32) -> f32 {
    (v - 0.5) * 24.0
}

impl GraphicEq {
    pub fn new(sr: f32) -> Self {
        let flat = || std::array::from_fn(|i| Biquad::peak_eq(sr, FREQS[i], Q, 0.0));
        let mut eq = Self {
            sr,
            left: flat(),
            right: flat(),
            last_bands: [-1.0; BANDS], // force first rebuild
            last_level: -1.0,
            level_lin: 1.0,
        };
        eq.rebuild(&[0.5; BANDS], 0.5);
        eq
    }

    fn rebuild(&mut self, bands: &[f32; BANDS], level: f32) {
        for i in 0..BANDS {
            let db = band_db(bands[i]);
            self.left[i] = Biquad::peak_eq(self.sr, FREQS[i], Q, db);
            self.right[i] = Biquad::peak_eq(self.sr, FREQS[i], Q, db);
        }
        // Output level: 0.5 = unity, ends are ±15 dB of make-up/trim.
        self.level_lin = db_to_lin((level - 0.5) * 30.0);
        self.last_bands = *bands;
        self.last_level = level;
    }

    /// Seven band faders (`b1`–`b7`, low → high) and an output `level`, all 0–1.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn process(
        &mut self,
        l: f32,
        r: f32,
        b1: f32,
        b2: f32,
        b3: f32,
        b4: f32,
        b5: f32,
        b6: f32,
        b7: f32,
        level: f32,
    ) -> (f32, f32) {
        let bands = [b1, b2, b3, b4, b5, b6, b7];
        if level != self.last_level
            || bands
                .iter()
                .zip(&self.last_bands)
                .any(|(&n, &o)| param_changed(n, o))
        {
            self.rebuild(&bands, level);
        }

        let mut lo = l;
        let mut ro = r;
        for i in 0..BANDS {
            lo = self.left[i].process(lo);
            ro = self.right[i].process(ro);
        }
        (lo * self.level_lin, ro * self.level_lin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const SR: f32 = 48_000.0;

    /// RMS of the settled tail of a mono sine driven through the (stereo) EQ at the
    /// given band/level settings. Both channels get the same signal, so the left
    /// tail is representative.
    fn band_rms(bands: [f32; BANDS], level: f32, freq: f32) -> f64 {
        let mut eq = GraphicEq::new(SR);
        let [b1, b2, b3, b4, b5, b6, b7] = bands;
        let mut sum = 0.0f64;
        let n = SR as usize;
        for i in 0..n {
            let x = (2.0 * PI * freq * i as f32 / SR).sin();
            let (yl, yr) = eq.process(x, x, b1, b2, b3, b4, b5, b6, b7, level);
            assert!(yl.is_finite() && yr.is_finite(), "GEQ non-finite at {i}");
            assert!(yl.abs() < 8.0, "GEQ unbounded: {yl}");
            if i >= n / 2 {
                sum += (yl * yl) as f64;
            }
        }
        (sum / (n / 2) as f64).sqrt()
    }

    /// Boosting a band must raise energy at that band's centre, and cutting it must
    /// lower it — one direct check per fader, so every slider is proven live.
    #[test]
    fn each_band_moves_its_own_frequency() {
        for (i, &f) in FREQS.iter().enumerate() {
            let mut boost = [0.5; BANDS];
            let mut cut = [0.5; BANDS];
            boost[i] = 1.0;
            cut[i] = 0.0;
            let up = band_rms(boost, 0.5, f);
            let down = band_rms(cut, 0.5, f);
            assert!(
                up > down * 1.2,
                "band {i} ({f} Hz) fader is dead: boost {up:.4} vs cut {down:.4}"
            );
        }
    }

    /// A boosted band must not bleed into a distant neighbour: pushing the 100 Hz
    /// fader should leave 10 kHz essentially untouched (the bands are independent).
    #[test]
    fn bands_are_reasonably_independent() {
        let mut low_boost = [0.5; BANDS];
        low_boost[0] = 1.0;
        let high_at_low_boost = band_rms(low_boost, 0.5, FREQS[BANDS - 1]);
        let high_flat = band_rms([0.5; BANDS], 0.5, FREQS[BANDS - 1]);
        assert!(
            (high_at_low_boost - high_flat).abs() < high_flat * 0.2,
            "100 Hz boost leaked into 10 kHz: {high_at_low_boost:.4} vs {high_flat:.4}"
        );
    }

    /// The output level fader must scale the whole signal: up is louder than down
    /// at a flat EQ setting.
    #[test]
    fn level_fader_scales_output() {
        let hot = band_rms([0.5; BANDS], 1.0, 1000.0);
        let quiet = band_rms([0.5; BANDS], 0.0, 1000.0);
        assert!(
            hot > quiet * 2.0,
            "level fader dead: {hot:.4} vs {quiet:.4}"
        );
    }

    /// A flat EQ (all faders centred) must pass the signal essentially unchanged —
    /// no net boost or cut, unity level.
    #[test]
    fn flat_setting_is_transparent() {
        let flat = band_rms([0.5; BANDS], 0.5, 1000.0);
        // A raw unit sine has RMS 1/√2 ≈ 0.707.
        assert!(
            (flat - 0.707).abs() < 0.05,
            "flat EQ is not transparent: {flat:.4}"
        );
    }
}
