use super::ir::{self, Texture};
use super::{BlendedCab, Cabinet};
use crate::dsp::biquad::Biquad;

/// Orange PPC412 4×12 with Celestion Vintage 30 speakers, multi-mic'd.
///
/// Same three-capture blend as the other cabs (close SM57 dynamic, close R121
/// ribbon, room mic — see [`BlendedCab`]). The PPC412 is a heavy, closed-back
/// birch-ply cab, so it reads as thick and chunky: a big low-mid "wall" of body,
/// a forward midrange grind and a smooth, slightly rolled-off top. Compared to the
/// Mesa (also V30s) the scoop is filled in — Orange is all about thick mids — and
/// the cabinet body modes ring a touch longer for that closed-back chest thump.
///
/// V30-in-birch close-mic (SM57) signature (the skeleton):
///   • Sub HP at 85 Hz (tight, closed-back low end)
///   • +4 dB low shelf at 110 Hz (PPC412 birch-ply low-end weight)
///   • -2 dB at 300 Hz (trim the boxiness, but keep more than the Mesa scoop)
///   • +5 dB at 600 Hz (Orange low-mid "wall" — the signature chunk)
///   • +3 dB at 1200 Hz (forward midrange grind)
///   • +5 dB at 3200 Hz (V30 presence spike, a touch lower/smoother than Mesa)
///   • -13 dB high shelf at 5200 Hz (closed-back cone rolloff)
///   • LP at 8500 Hz (fizz cut)
pub struct OrangeCab {
    inner: BlendedCab,
}

// Left/right speaker textures: slightly different reflection times and modes so
// the two channels decorrelate. The closed-back birch cab gives tight, fairly
// hot early taps and a deep ~80 Hz "chest thump" body mode that rings long for
// weight; the ~3.3 kHz mode is the V30 breakup.
const TEX_L: Texture = Texture {
    predelay: 0,
    reflections: &[
        (0.28, -0.30),
        (0.88, 0.21),
        (1.75, -0.11),
        (3.20, 0.07),
        (6.50, -0.052),
        (11.00, 0.038),
        (17.80, -0.025),
        (20.50, 0.015),
    ],
    modes: &[
        (80.0, 125.0, 0.006),
        (100.0, 105.0, 0.007),
        (3300.0, 4.0, 0.1),
    ],
};
const TEX_R: Texture = Texture {
    predelay: 2,
    reflections: &[
        (0.32, -0.27),
        (0.98, 0.22),
        (1.95, -0.10),
        (3.50, 0.06),
        (7.10, -0.048),
        (11.80, 0.035),
        (18.90, -0.023),
        (20.50, 0.014),
    ],
    modes: &[
        (85.0, 130.0, 0.006),
        (107.0, 110.0, 0.007),
        (3450.0, 4.0, 0.10),
    ],
};

// Room-mic textures: extra pre-delay (distance) and denser, later reflections so
// the room mic reads as a few feet back in the room rather than on the grille.
const ROOM_TEX_L: Texture = Texture {
    predelay: 115,
    reflections: &[
        (2.60, 0.21),
        (5.50, -0.17),
        (9.30, 0.14),
        (13.50, -0.10),
        (16.00, 0.078),
        (18.50, -0.058),
    ],
    modes: &[(84.0, 122.0, 0.006), (175.0, 95.0, 0.005)],
};
const ROOM_TEX_R: Texture = Texture {
    predelay: 142,
    reflections: &[
        (3.00, 0.19),
        (6.30, -0.16),
        (10.50, 0.13),
        (13.50, -0.095),
        (15.50, 0.072),
        (18.00, -0.053),
    ],
    modes: &[(88.0, 127.0, 0.006), (185.0, 100.0, 0.005)],
};

impl OrangeCab {
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

    /// SM57 close-mic: the thick, mid-forward Orange voicing (the original skeleton).
    fn voicing_sm57(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 85.0, 0.9),
            // The Orange "wall of mids" was so forward (+5 @ 600, +3 @ 1200) that the
            // upper-mid single notes rode far louder than the low-mids. Pulled back
            // (+5→+2.5 @ 600, +3→+1.5 @ 1200) with a shallower 300 dip and more low
            // shelf so the cab stays thick and chunky but level across the neck.
            Biquad::low_shelf(sr, 110.0, 4.5),
            Biquad::peak_eq(sr, 300.0, 1.5, -1.0),
            Biquad::peak_eq(sr, 600.0, 1.2, 2.5),
            Biquad::peak_eq(sr, 1200.0, 1.2, 1.5),
            // V30 presence, broadened and tamed (Q 2.0→1.4, +5→+3.5 dB) so the
            // forward Orange mid grind stays but the top loses its ice-pick edge.
            Biquad::peak_eq(sr, 3200.0, 1.4, 3.5),
            Biquad::high_shelf(sr, 5200.0, -13.0),
            Biquad::lowpass(sr, 8500.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }

    /// R121 ribbon close-mic: even thicker low-mids, softer presence, silky top.
    fn voicing_ribbon(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 80.0, 0.9),
            Biquad::low_shelf(sr, 150.0, 4.0),
            Biquad::peak_eq(sr, 300.0, 1.5, -1.5),
            Biquad::peak_eq(sr, 600.0, 1.0, 4.0),
            Biquad::peak_eq(sr, 1200.0, 1.0, 2.0),
            Biquad::peak_eq(sr, 3000.0, 1.6, 2.5), // gentler, lower presence
            Biquad::high_shelf(sr, 4400.0, -15.0), // ribbon HF rolloff
            Biquad::lowpass(sr, 6500.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }

    /// Room mic: a darker, distance-coloured version of the cab voicing.
    fn voicing_room(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 95.0, 0.8),
            Biquad::low_shelf(sr, 150.0, 2.0),
            Biquad::peak_eq(sr, 350.0, 1.2, -1.5),
            Biquad::peak_eq(sr, 700.0, 1.0, 2.5),
            Biquad::high_shelf(sr, 3900.0, -10.0),
            Biquad::lowpass(sr, 5400.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }
}

impl Cabinet for OrangeCab {
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
        analysis::assert_plausible("orange close L", sr, &TEX_L);
        analysis::assert_plausible("orange close R", sr, &TEX_R);
        analysis::assert_plausible("orange room L", sr, &ROOM_TEX_L);
        analysis::assert_plausible("orange room R", sr, &ROOM_TEX_R);
        analysis::assert_lr_symmetry("orange close", &TEX_L, &TEX_R);
        analysis::assert_lr_symmetry("orange room", &ROOM_TEX_L, &ROOM_TEX_R);
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
        check!("orange close L", OrangeCab::voicing_sm57, &TEX_L);
        check!("orange close R", OrangeCab::voicing_sm57, &TEX_R);
        check!("orange room L", OrangeCab::voicing_room, &ROOM_TEX_L);
        check!("orange room R", OrangeCab::voicing_room, &ROOM_TEX_R);
    }
}
