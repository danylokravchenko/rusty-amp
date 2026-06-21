use super::Cabinet;
use super::ir::{self, Texture};
use crate::dsp::biquad::Biquad;
use crate::dsp::conv::FirConvolver;

/// Marshall 4×12 with Celestion Greenback speakers, SM57 close-mic.
///
/// As with the Mesa cab, the Greenback voicing is the magnitude skeleton of a
/// synthesised impulse response; convolution adds the early-reflection comb and
/// cone ring. Greenbacks are inherently smoother and warmer than V30s, so the
/// reflections are gentler and the breakup mode sits lower.
///
/// Greenback EQ signature (the skeleton):
///   • Sub HP at 80 Hz (slightly looser than V30 — GB has more low-end air)
///   • +2 dB low shelf at 120 Hz (GB low warmth)
///   • -3 dB at 250 Hz (reduce muddiness below the GB mid-body peak)
///   • +4 dB at 800 Hz (Greenback signature mid warmth — the "vintage" honk)
///   • -2 dB at 1500 Hz (upper-mid smoothness — GBs are less aggressive than V30s)
///   • +5 dB at 2500 Hz (GB presence peak — warmer/lower than V30's 3.5 kHz)
///   • -10 dB high shelf at 5000 Hz (softer cone rolloff vs V30)
///   • LP at 8 kHz (fizz cut — GBs are inherently smoother on top)
pub struct MarshallCab {
    sr: f32,
    conv_l: FirConvolver,
    conv_r: FirConvolver,
    mic_l: Biquad,
    mic_r: Biquad,
    last_mic_pos: f32,
}

// Gentler, slightly later reflections than the V30 (smoother Greenback cone),
// breakup mode lower at ~2.5 kHz. The Greenback has more low-end air than a V30,
// so the late room taps run a touch longer/hotter and the deep ~66 Hz body mode
// is given a long decay for that warm, three-dimensional Marshall bloom.
const TEX_L: Texture = Texture {
    predelay: 0,
    reflections: &[
        (0.30, -0.26),
        (0.95, 0.17),
        (1.90, -0.09),
        (3.40, 0.05),
        (7.10, -0.050),
        (12.40, 0.038),
        (20.00, -0.024),
        (30.00, 0.015),
    ],
    modes: &[(66.0, 240.0, 0.12), (90.0, 165.0, 0.16), (2500.0, 5.0, 0.10)],
};
const TEX_R: Texture = Texture {
    predelay: 2,
    reflections: &[
        (0.34, -0.23),
        (1.05, 0.18),
        (2.10, -0.08),
        (3.70, 0.05),
        (7.80, -0.046),
        (13.30, 0.035),
        (21.50, -0.022),
        (32.00, 0.014),
    ],
    modes: &[(70.0, 250.0, 0.11), (96.0, 175.0, 0.15), (2600.0, 5.0, 0.09)],
};

impl MarshallCab {
    pub fn new(sr: f32) -> Self {
        let len = ir::ir_len(sr);
        let cap = len + 1;
        let mut conv_l = FirConvolver::new(cap);
        let mut conv_r = FirConvolver::new(cap);
        conv_l.load(&ir::synth(sr, len, &mut Self::voicing(sr), &TEX_L));
        conv_r.load(&ir::synth(sr, len, &mut Self::voicing(sr), &TEX_R));
        Self {
            sr,
            conv_l,
            conv_r,
            mic_l: Biquad::high_shelf(sr, 5000.0, 0.0),
            mic_r: Biquad::high_shelf(sr, 5000.0, 0.0),
            last_mic_pos: -1.0,
        }
    }

    /// The Greenback EQ chain as a per-sample voicing function (the IR skeleton).
    fn voicing(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 80.0, 0.8),
            Biquad::low_shelf(sr, 120.0, 2.0),
            Biquad::peak_eq(sr, 250.0, 1.5, -3.0),
            Biquad::peak_eq(sr, 800.0, 1.5, 4.0),
            Biquad::peak_eq(sr, 1500.0, 1.5, -2.0),
            Biquad::peak_eq(sr, 2500.0, 1.8, 5.0),
            Biquad::high_shelf(sr, 5000.0, -10.0),
            Biquad::lowpass(sr, 8000.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }
}

impl Cabinet for MarshallCab {
    #[inline]
    fn process(&mut self, sample: f32, mic_pos: f32) -> (f32, f32) {
        if (mic_pos - self.last_mic_pos).abs() > 0.001 {
            let db = (mic_pos - 0.5) * 12.0;
            self.mic_l = Biquad::high_shelf(self.sr, 5000.0, db);
            self.mic_r = Biquad::high_shelf(self.sr, 5000.0, db);
            self.last_mic_pos = mic_pos;
        }
        let l = self.mic_l.process(self.conv_l.process(sample));
        let r = self.mic_r.process(self.conv_r.process(sample));
        (l, r)
    }
}
