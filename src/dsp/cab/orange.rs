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
/// Voiced against measured commercial 4×12 captures (see the note in the Mesa
/// `voicing_sm57`):
///   • Resonant sub HP at 64 Hz (tight, closed-back low end — but with real depth)
///   • +6 dB low shelf at 110 Hz + a +6 dB hump at 118 Hz (the "chest thump")
///   • +3.5 dB wide mound at 230 Hz + +2.5 dB at 550 Hz (low-mid "wall" / body)
///   • -2.5 dB at 1450 Hz over a −3 dB shelf from 1.35 kHz (mid pocket — the
///     grind stays but doesn't crowd the body)
///   • +3.5 dB at 3200 Hz and +2 dB at 4.4 kHz (V30 presence, a touch
///     lower/smoother than Mesa, held through the 3–5 kHz band)
///   • -13 dB high shelf at 5200 Hz (closed-back cone rolloff)
///   • LP at 8500 Hz (fizz cut)
pub struct OrangeCab {
    inner: BlendedCab,
}

// Left/right speaker textures: slightly different reflection times and modes so
// the two channels decorrelate. The closed-back birch cab gives tight, fairly
// hot early taps, timed so the comb notches land in the mid pocket (see the Mesa
// texture note). The two low modes near 100–128 Hz add the closed-back thump
// ring where the direct sound is strong; the ~3.3 kHz mode is the V30 breakup.
const TEX_L: Texture = Texture {
    predelay: 0,
    reflections: &[
        (0.28, -0.30),
        (0.62, 0.21),
        (1.24, -0.11),
        (3.20, 0.078),
        (6.50, -0.072),
        (11.00, 0.060),
        (14.60, -0.048),
        (17.80, 0.038),
        (20.50, 0.029),
    ],
    modes: &[
        (100.0, 95.0, 0.004),
        (125.0, 85.0, 0.004),
        (3300.0, 4.0, 0.1),
    ],
};
const TEX_R: Texture = Texture {
    predelay: 2,
    reflections: &[
        (0.32, -0.27),
        (0.66, 0.22),
        (1.32, -0.10),
        (3.50, 0.074),
        (7.10, -0.068),
        (11.80, 0.056),
        (15.40, -0.045),
        (18.90, 0.036),
        (20.50, 0.027),
    ],
    modes: &[
        (103.0, 97.0, 0.004),
        (128.0, 87.0, 0.004),
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
    modes: &[(84.0, 130.0, 0.006), (175.0, 95.0, 0.005)],
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
    modes: &[(88.0, 135.0, 0.006), (185.0, 100.0, 0.005)],
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
            // Resonant rise into a ~118 Hz "chest thump" hump, then the broad
            // body plateau over a shallow mid pocket (see the Mesa `voicing_sm57`
            // note). The bottom octave runs hotter than the other cabs — the
            // closed-back thump also has to carry the low fundamentals over the
            // Randall's dense overtones (see `fundamental_is_not_buried…`).
            Biquad::highpass(sr, 64.0, 1.2),
            Biquad::low_shelf(sr, 110.0, 6.0),
            Biquad::peak_eq(sr, 118.0, 1.4, 6.0),
            Biquad::peak_eq(sr, 230.0, 0.8, 3.5),
            Biquad::peak_eq(sr, 550.0, 0.9, 2.5),
            Biquad::peak_eq(sr, 1450.0, 0.55, -2.5),
            // Downward 0.5–2 kHz tilt (see the Mesa voicing note).
            Biquad::high_shelf(sr, 1350.0, -3.0),
            // V30 presence, broadened and tamed (Q 2.0→1.4, +5→+3.5 dB) so the
            // forward Orange mid grind stays but the top loses its ice-pick edge.
            Biquad::peak_eq(sr, 3200.0, 1.1, 3.0),
            Biquad::peak_eq(sr, 4400.0, 1.5, 3.0),
            Biquad::high_shelf(sr, 6200.0, -13.0),
            Biquad::lowpass(sr, 8500.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }

    /// R121 ribbon close-mic: even thicker low-mids, softer presence, silky top.
    fn voicing_ribbon(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 72.0, 1.2),
            Biquad::low_shelf(sr, 150.0, 2.5),
            Biquad::peak_eq(sr, 120.0, 1.1, 5.5), // low resonant hump (cab depth)
            Biquad::peak_eq(sr, 220.0, 0.7, 5.0), // broad low-mid body mound
            Biquad::peak_eq(sr, 550.0, 0.9, 2.5),
            Biquad::peak_eq(sr, 1450.0, 0.55, -2.5),
            // Downward 0.5–2 kHz tilt (see the Mesa voicing note).
            Biquad::high_shelf(sr, 1350.0, -3.0),
            Biquad::peak_eq(sr, 3000.0, 1.6, 2.5), // gentler, lower presence
            Biquad::high_shelf(sr, 4400.0, -15.0), // ribbon HF rolloff
            Biquad::lowpass(sr, 6500.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }

    /// Room mic: a darker, distance-coloured version of the cab voicing.
    fn voicing_room(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 78.0, 0.8),
            Biquad::low_shelf(sr, 150.0, 3.0),
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
