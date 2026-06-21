use super::Cabinet;
use super::ir::{self, Texture};
use crate::dsp::biquad::Biquad;
use crate::dsp::conv::FirConvolver;

/// Mesa/Boogie 4×12 with Celestion Vintage 30 speakers, SM57 close-mic.
///
/// The V30 voicing below is realised as the magnitude skeleton of a synthesised
/// impulse response (see [`ir`]); convolution then adds the early-reflection comb
/// and cone-resonance ring of a real miked cab. Two decorrelated textures feed
/// the left/right channels for a natural stereo image.
///
/// V30 EQ signature (the skeleton):
///   • Sub HP at 80 Hz with slight resonance (ported cab alignment)
///   • +3 dB low shelf at 100 Hz (speaker low-end weight)
///   • -4 dB at 300 Hz (cardboard boxiness notch)
///   • -5 dB at 400 Hz (honky lower-mid cut — V30 mid scoop)
///   • +2 dB at 800 Hz (low-mid body / pick attack)
///   • +7 dB at 3500 Hz (V30 signature presence spike)
///   • -14 dB high shelf at 5500 Hz (speaker cone rolloff)
///   • LP at 9 kHz (fizz cut + cone break-up noise removal)
pub struct MesaCab {
    sr: f32,
    conv_l: FirConvolver,
    conv_r: FirConvolver,
    mic_l: Biquad,
    mic_r: Biquad,
    last_mic_pos: f32,
}

// Left/right speaker textures: slightly different reflection times and modes so
// the two channels decorrelate (stereo width) without smearing a mono sum.
//
// Early taps (< 4 ms) are the cone-to-grille / panel comb that colours the body;
// the later taps (6–28 ms) are cabinet-edge and near-wall reflections that put
// the speaker in a space and give the note depth and air. The low modal pair
// (a deep ~75 Hz cabinet "thump" plus the ~95 Hz cone resonance) blooms and
// rings out across the long IR for weight; the ~3.4 kHz mode is the V30 breakup.
const TEX_L: Texture = Texture {
    predelay: 0,
    reflections: &[
        (0.27, -0.32),
        (0.85, 0.20),
        (1.70, -0.12),
        (3.10, 0.07),
        (6.30, -0.055),
        (10.80, 0.040),
        (17.50, -0.026),
        (26.00, 0.015),
    ],
    modes: &[(74.0, 210.0, 0.11), (95.0, 150.0, 0.18), (3400.0, 4.0, 0.12)],
};
const TEX_R: Texture = Texture {
    predelay: 2,
    reflections: &[
        (0.31, -0.28),
        (0.95, 0.22),
        (1.90, -0.10),
        (3.40, 0.06),
        (6.90, -0.050),
        (11.60, 0.037),
        (18.80, -0.024),
        (28.00, 0.014),
    ],
    modes: &[(79.0, 220.0, 0.10), (102.0, 160.0, 0.17), (3550.0, 4.0, 0.11)],
};

impl MesaCab {
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

    /// The V30 EQ chain as a per-sample voicing function (the IR skeleton).
    fn voicing(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 80.0, 0.9),
            Biquad::low_shelf(sr, 100.0, 3.0),
            Biquad::peak_eq(sr, 300.0, 1.8, -4.0),
            Biquad::peak_eq(sr, 400.0, 1.5, -5.0),
            Biquad::peak_eq(sr, 800.0, 1.5, 2.0),
            Biquad::peak_eq(sr, 3500.0, 2.0, 7.0),
            Biquad::high_shelf(sr, 5500.0, -14.0),
            Biquad::lowpass(sr, 9000.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }
}

impl Cabinet for MesaCab {
    #[inline]
    fn process(&mut self, sample: f32, mic_pos: f32) -> (f32, f32) {
        if (mic_pos - self.last_mic_pos).abs() > 0.001 {
            // 0 = edge (off-axis, dark), 1 = center (on-axis, bright)
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
