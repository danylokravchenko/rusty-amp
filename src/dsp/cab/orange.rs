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
///   • Resonant sub HP at 74 Hz (tight, closed-back low end — but with real depth)
///   • +5 dB low shelf at 110 Hz + a wide +6 dB hump at 115 Hz (the "chest thump",
///     carried down to the 82 Hz low-E fundamental)
///   • +3.8 dB wide mound at 230 Hz + +5.5 dB at 550 Hz and +2.5 dB at 800 Hz
///     (low-mid "wall" / body)
///   • −4.5 dB at 1300 Hz over a −1 dB shelf from 1.35 kHz (mid pocket — the
///     grind stays but doesn't crowd the body)
///   • +3.5 dB wide mound at 2.9 kHz and +5.5 dB at 4.5 kHz (V30 presence, a
///     touch lower/smoother than Mesa, held through the 3–5 kHz band)
///   • −19 dB high shelf at 6600 Hz (closed-back cone rolloff)
///   • 4th-order LP at 7 kHz (fizz cut — real captures carry no 8 kHz energy)
pub struct OrangeCab {
    inner: BlendedCab,
}

// Close-mic texture, shared by both channels (references are mono — see the
// Mesa texture note; only the scatter seeds differ per channel, the room pair
// below carries the true stereo decorrelation). The closed-back birch cab gives
// tight, fairly hot early taps, timed so the comb notches land in the mid pocket.
// The two low modes near 100–125 Hz add the closed-back thump ring where the
// direct sound is strong, T60s kept short of the note (gated references — see
// the Mesa note); the ~3.3 kHz mode is the V30 breakup.
// Dense 6–21 ms tail (see the Mesa TEX_REFL note): real captures are diffuse
// there, not a few discrete echoes.
// Early-tap gains kept small (~0.1) — see the Mesa TEX_REFL note: hot early
// taps are a voicing move, not texture.
const TEX_REFL: &[(f32, f32)] = &[
    (0.28, -0.15),
    (0.62, 0.11),
    (1.24, -0.095),
    (3.20, 0.085),
    (6.50, -0.072),
    (8.70, 0.066),
    (11.00, -0.060),
    (12.80, 0.054),
    (14.60, -0.048),
    (16.20, 0.042),
    (17.80, -0.038),
    (19.20, 0.033),
    (20.50, -0.029),
];
// Breakup mode at texture scale — see the Mesa TEX_MODES note.
const TEX_MODES: &[(f32, f32, f32)] = &[
    (100.0, 55.0, 0.004),
    (125.0, 50.0, 0.004),
    (3300.0, 4.0, 0.045),
];
// V30 breakup scatter plus the mid reflection-ripple cluster (see the Mesa
// scatter note). Gains run lighter than the other cabs': this seed pair draws
// hotter modes from the LCG, and at the shared levels the decorrelated ripple
// pushed L/R correlation down to 0.69 and the 0.5–1 kHz ripple past the refs.
const SCATTER_L: &[ir::Scatter] = &[
    ir::Scatter {
        seed: 31,
        count: 22,
        band: (2100.0, 6800.0),
        t60_ms: (5.0, 12.0),
        gain: 0.018,
    },
    ir::Scatter {
        seed: 33,
        count: 24,
        band: (550.0, 2300.0),
        t60_ms: (6.0, 16.0),
        gain: 0.016,
    },
];
const SCATTER_R: &[ir::Scatter] = &[
    ir::Scatter {
        seed: 32,
        count: 22,
        band: (2100.0, 6800.0),
        t60_ms: (5.0, 12.0),
        gain: 0.018,
    },
    ir::Scatter {
        seed: 34,
        count: 24,
        band: (550.0, 2300.0),
        t60_ms: (6.0, 16.0),
        gain: 0.016,
    },
];
const TEX_L: Texture = Texture {
    predelay: 0,
    reflections: TEX_REFL,
    modes: TEX_MODES,
    scatter: SCATTER_L,
};
const TEX_R: Texture = Texture {
    predelay: 0,
    reflections: TEX_REFL,
    modes: TEX_MODES,
    scatter: SCATTER_R,
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
    modes: &[(84.0, 65.0, 0.005), (175.0, 55.0, 0.004)],
    scatter: &[],
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
    modes: &[(88.0, 65.0, 0.005), (185.0, 55.0, 0.004)],
    scatter: &[],
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
            Biquad::highpass(sr, 74.0, 1.2),
            // Shelf trimmed 6 → 5 dB (measured hot at 80 Hz vs the references —
            // chest thump, not sub boom). The hump is wider than the other
            // cabs' (Q 1.0): its lower skirt is what carries the 82 Hz low-E
            // fundamental over the Randall's dense overtones (see
            // `fundamental_is_not_buried…`), now that the shelf sits lower.
            Biquad::low_shelf(sr, 110.0, 5.0),
            Biquad::peak_eq(sr, 115.0, 1.0, 6.0),
            Biquad::peak_eq(sr, 230.0, 0.8, 3.8),
            // Body carried up through ~700 Hz with the pocket at 1.3 kHz: real
            // captures hold their 0.5–1 kHz level and dip in the 1–2 kHz octave
            // (see mesa.rs `voicing_sm57`). Widened (Q 0.8 → 0.6) and joined by
            // an 800 Hz filler: this cab measured −4…−7 dB through 500–800 Hz
            // against the refs.
            Biquad::peak_eq(sr, 550.0, 0.6, 5.5),
            Biquad::peak_eq(sr, 800.0, 1.2, 2.5),
            // Pocket narrowed and tilt eased (see the Mesa voicing note): the
            // old shape dug a measured −6.4 dB hole at 2.5 kHz vs the refs.
            Biquad::peak_eq(sr, 1300.0, 1.15, -4.5),
            Biquad::high_shelf(sr, 1350.0, -1.0),
            // V30 presence, a wide low-centred mound (like the Mesa's, a touch
            // lower still) so the forward Orange mid grind leads without an
            // ice-pick edge; the 4.5 kHz peak keeps the 4–5 kHz level real
            // captures hold before the rolloff.
            Biquad::peak_eq(sr, 2900.0, 0.75, 3.5),
            Biquad::peak_eq(sr, 4500.0, 1.3, 5.5),
            // Real V30 captures carry no 8 kHz energy (measured +12 dB of fizz
            // vs the refs with the old single pole).
            Biquad::high_shelf(sr, 6600.0, -19.0),
            Biquad::lowpass(sr, 7000.0, 0.707),
            Biquad::lowpass(sr, 7000.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }

    /// R121 ribbon close-mic: even thicker low-mids, softer presence, silky top.
    fn voicing_ribbon(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 74.0, 1.2),
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
            scatter: t.scatter,
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
