use super::ir::{self, Texture};
use super::{BlendedCab, Cabinet};
use crate::dsp::biquad::Biquad;

/// Mesa/Boogie 4×12 with Celestion Vintage 30 speakers, multi-mic'd.
///
/// Three mic captures are synthesised and blended (see [`BlendedCab`]): a close
/// SM57 dynamic (the bright, present backbone), a close R121 ribbon (darker,
/// fuller low-mids, silky top) and a room mic for depth. Each capture realises a
/// voiced EQ "skeleton" plus a reflection texture; the room mic adds pre-delay and
/// denser late reflections for a sense of air.
///
/// V30 close-mic (SM57) signature:
///   • Sub HP at 80 Hz with slight resonance (ported cab alignment)
///   • +3 dB low shelf at 100 Hz (speaker low-end weight)
///   • -4 dB at 300 Hz (cardboard boxiness notch)
///   • -5 dB at 400 Hz (honky lower-mid cut — V30 mid scoop)
///   • +2 dB at 800 Hz (low-mid body / pick attack)
///   • +7 dB at 3500 Hz (V30 signature presence spike)
///   • -14 dB high shelf at 5500 Hz (speaker cone rolloff)
///   • LP at 9 kHz (fizz cut + cone break-up noise removal)
pub struct MesaCab {
    inner: BlendedCab,
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
    modes: &[
        (74.0, 210.0, 0.11),
        (95.0, 150.0, 0.18),
        (3400.0, 4.0, 0.12),
    ],
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
    modes: &[
        (79.0, 220.0, 0.10),
        (102.0, 160.0, 0.17),
        (3550.0, 4.0, 0.11),
    ],
};

// Room-mic textures: extra pre-delay (distance) and denser, later reflections so
// the room mic reads as a few feet back in the room rather than on the grille.
const ROOM_TEX_L: Texture = Texture {
    predelay: 110,
    reflections: &[
        (2.50, 0.22),
        (5.40, -0.18),
        (9.10, 0.14),
        (14.00, -0.11),
        (20.00, 0.08),
        (28.00, -0.06),
    ],
    modes: &[(82.0, 240.0, 0.10), (180.0, 120.0, 0.06)],
};
const ROOM_TEX_R: Texture = Texture {
    predelay: 138,
    reflections: &[
        (2.90, 0.20),
        (6.10, -0.17),
        (10.20, 0.13),
        (15.40, -0.10),
        (22.00, 0.075),
        (30.00, -0.055),
    ],
    modes: &[(86.0, 250.0, 0.09), (190.0, 125.0, 0.055)],
};

impl MesaCab {
    pub fn new(sr: f32) -> Self {
        let len = ir::ir_len(sr);
        let synth = |v: &mut dyn FnMut(f32) -> f32, t: &Texture| ir::synth(sr, len, v, t);
        let irs = [
            synth(&mut Self::voicing_sm57(sr), &TEX_L),
            synth(&mut Self::voicing_sm57(sr), &TEX_R),
            synth(&mut Self::voicing_ribbon(sr), &TEX_L),
            synth(&mut Self::voicing_ribbon(sr), &TEX_R),
            synth(&mut Self::voicing_room(sr), &ROOM_TEX_L),
            synth(&mut Self::voicing_room(sr), &ROOM_TEX_R),
        ];
        Self {
            inner: BlendedCab::new(sr, irs),
        }
    }

    /// SM57 close-mic: the bright, present V30 voicing (the original skeleton).
    fn voicing_sm57(sr: f32) -> impl FnMut(f32) -> f32 {
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

    /// R121 ribbon close-mic: fuller low-mids, softer presence, silky top rolloff.
    fn voicing_ribbon(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 75.0, 0.9),
            Biquad::low_shelf(sr, 140.0, 3.5),
            Biquad::peak_eq(sr, 300.0, 1.8, -3.0),
            Biquad::peak_eq(sr, 500.0, 1.2, -2.0),
            Biquad::peak_eq(sr, 1100.0, 1.0, 2.0), // ribbon body
            Biquad::peak_eq(sr, 3200.0, 1.6, 2.5), // gentler, lower presence
            Biquad::high_shelf(sr, 4500.0, -16.0), // ribbon HF rolloff
            Biquad::lowpass(sr, 6500.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }

    /// Room mic: a darker, distance-coloured version of the cab voicing.
    fn voicing_room(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 95.0, 0.8),
            Biquad::low_shelf(sr, 150.0, 1.5),
            Biquad::peak_eq(sr, 400.0, 1.2, -3.0),
            Biquad::peak_eq(sr, 1200.0, 1.0, 1.5),
            Biquad::high_shelf(sr, 4000.0, -10.0),
            Biquad::lowpass(sr, 5500.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }
}

impl Cabinet for MesaCab {
    #[inline]
    fn process(&mut self, sample: f32, mic_pos: f32, blend: f32, room: f32) -> (f32, f32) {
        self.inner.process(sample, mic_pos, blend, room)
    }
}
