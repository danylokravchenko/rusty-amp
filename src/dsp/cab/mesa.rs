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
/// V30 close-mic (SM57) signature, voiced against measured commercial 4×12
/// captures (see the note in `voicing_sm57`):
///   • Resonant sub HP at 72 Hz (ported cab alignment)
///   • +3 dB low shelf at 100 Hz + a +6.5 dB resonant hump at 120 Hz (cab depth)
///   • +5 dB wide mound at 220 Hz and +3 dB at 500 Hz (low-mid body plateau)
///   • −2.5 dB wide dip at 1150 Hz (the mid "pocket" of a real capture)
///   • +4 dB at 3500 Hz (V30 signature presence)
///   • -14 dB high shelf at 5500 Hz (speaker cone rolloff)
///   • LP at 9 kHz (fizz cut + cone break-up noise removal)
pub struct MesaCab {
    inner: BlendedCab,
}

// Left/right speaker textures: slightly different reflection times and modes so
// the two channels decorrelate (stereo width) without smearing a mono sum.
//
// Early taps (< 4 ms) are the cone-to-grille / panel comb that colours the body —
// timed so their comb notches land in the 800 Hz–2 kHz mid pocket rather than in
// the 400–600 Hz body; the later taps (6–28 ms) are cabinet-edge and near-wall
// reflections that put the speaker in a space and give the note depth and air.
// The two low modes near 100–120 Hz add a subtle thump ring on top of the EQ hump
// (they sit where the direct sound is strong: an additive resonance placed where
// the direct path is weak phase-cancels it just above resonance and carves a
// notch instead of adding depth); the ~3.4 kHz mode is the V30 breakup.
const TEX_L: Texture = Texture {
    predelay: 0,
    reflections: &[
        (0.27, -0.32),
        (0.62, 0.20),
        (1.24, -0.12),
        (3.10, 0.07),
        (6.30, -0.055),
        (10.80, 0.040),
        (17.50, -0.026),
        (20.50, 0.015),
    ],
    modes: &[
        (98.0, 55.0, 0.0025),
        (118.0, 50.0, 0.0025),
        (3400.0, 4.0, 0.1),
    ],
};
const TEX_R: Texture = Texture {
    predelay: 2,
    reflections: &[
        (0.31, -0.28),
        (0.66, 0.22),
        (1.32, -0.10),
        (3.40, 0.06),
        (6.90, -0.050),
        (11.60, 0.037),
        (18.80, -0.024),
        (20.50, 0.014),
    ],
    modes: &[
        (100.0, 57.0, 0.0025),
        (122.0, 52.0, 0.0025),
        (3550.0, 4.0, 0.10),
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
        (16.50, 0.08),
        (18.50, -0.06),
    ],
    modes: &[(82.0, 130.0, 0.006), (180.0, 95.0, 0.005)],
};
const ROOM_TEX_R: Texture = Texture {
    predelay: 138,
    reflections: &[
        (2.90, 0.20),
        (6.10, -0.17),
        (10.20, 0.13),
        (14.00, -0.10),
        (16.00, 0.075),
        (18.00, -0.055),
    ],
    modes: &[(86.0, 135.0, 0.006), (190.0, 100.0, 0.005)],
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
            // The low end is voiced to the shape real close-mic'd 4×12 captures
            // (e.g. God's Cab) measure: a steep resonant rise into a big ~120 Hz
            // hump — a shelf alone is flat below its corner; the hump is what
            // reads as "deep" — then a broad +5…+10 dB body plateau from ~120 to
            // ~600 Hz relative to the 800 Hz–2 kHz band, which instead carries a
            // wide, shallow pocket. That plateau-vs-pocket tilt, not sub-bass, is
            // what makes a capture sound deep and juicy.
            Biquad::highpass(sr, 72.0, 1.2),
            Biquad::low_shelf(sr, 100.0, 3.0),
            Biquad::peak_eq(sr, 120.0, 1.1, 6.5),
            Biquad::peak_eq(sr, 220.0, 0.7, 5.0),
            Biquad::peak_eq(sr, 500.0, 0.8, 3.0),
            Biquad::peak_eq(sr, 1150.0, 0.7, -2.5),
            // V30 presence: broadened (Q 2.0→1.3) and tamed (+7→+4 dB). The narrow
            // +7 spike sat exactly on the 2–5 kHz "ice-pick" band and made high
            // notes shrill; a gentler, wider lift keeps the V30 bite without harsh.
            Biquad::peak_eq(sr, 3500.0, 1.3, 4.0),
            Biquad::high_shelf(sr, 5500.0, -14.0),
            Biquad::lowpass(sr, 9000.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }

    /// R121 ribbon close-mic: fuller low-mids, softer presence, silky top rolloff.
    fn voicing_ribbon(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 70.0, 1.2),
            Biquad::low_shelf(sr, 140.0, 2.5),
            Biquad::peak_eq(sr, 115.0, 1.1, 5.5), // low resonant hump (cab depth)
            Biquad::peak_eq(sr, 210.0, 0.7, 5.0), // broad low-mid body mound
            Biquad::peak_eq(sr, 500.0, 0.9, 2.5),
            Biquad::peak_eq(sr, 1150.0, 0.8, -1.5), // mid pocket
            Biquad::peak_eq(sr, 3200.0, 1.6, 2.5),  // gentler, lower presence
            Biquad::high_shelf(sr, 4500.0, -16.0),  // ribbon HF rolloff
            Biquad::lowpass(sr, 6500.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }

    /// Room mic: a darker, distance-coloured version of the cab voicing.
    fn voicing_room(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 78.0, 0.8),
            Biquad::low_shelf(sr, 150.0, 2.5),
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

#[cfg(test)]
mod tests {
    use super::*;
    use ir::analysis;

    #[test]
    fn textures_are_plausible_and_symmetric() {
        let sr = 48_000.0;
        analysis::assert_plausible("mesa close L", sr, &TEX_L);
        analysis::assert_plausible("mesa close R", sr, &TEX_R);
        analysis::assert_plausible("mesa room L", sr, &ROOM_TEX_L);
        analysis::assert_plausible("mesa room R", sr, &ROOM_TEX_R);
        analysis::assert_lr_symmetry("mesa close", &TEX_L, &TEX_R);
        analysis::assert_lr_symmetry("mesa room", &ROOM_TEX_L, &ROOM_TEX_R);
    }

    #[test]
    fn modes_are_realized_in_the_rendered_ir() {
        let sr = 48_000.0;
        let len = ir::ir_len(sr);
        let strip = |t: &Texture| Texture {
            predelay: t.predelay,
            reflections: t.reflections,
            modes: &[],
        };
        macro_rules! check {
            ($tag:expr, $voicing:expr, $tex:expr) => {{
                let full = ir::synth(sr, len, &mut $voicing(sr), $tex);
                let bare = ir::synth(sr, len, &mut $voicing(sr), &strip($tex));
                analysis::assert_modes_realized($tag, &full, &bare, sr, $tex);
            }};
        }
        check!("mesa close L", MesaCab::voicing_sm57, &TEX_L);
        check!("mesa close R", MesaCab::voicing_sm57, &TEX_R);
        check!("mesa room L", MesaCab::voicing_room, &ROOM_TEX_L);
        check!("mesa room R", MesaCab::voicing_room, &ROOM_TEX_R);
    }
}
