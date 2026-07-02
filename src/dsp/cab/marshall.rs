use super::ir::{self, Texture};
use super::{BlendedCab, Cabinet};
use crate::dsp::biquad::Biquad;

/// Marshall 4×12 with Celestion Greenback speakers, multi-mic'd.
///
/// Like the Mesa cab this blends three mic captures (close SM57 dynamic, close
/// R121 ribbon, and a room mic — see [`BlendedCab`]). Greenbacks are inherently
/// smoother and warmer than V30s, so the reflections are gentler and the breakup
/// mode sits lower.
///
/// Greenback close-mic (SM57) signature (the skeleton), voiced against measured
/// commercial 4×12 captures (see the note in the Mesa `voicing_sm57`):
///   • Resonant sub HP at 66 Hz (looser than V30 — GB has more low-end air)
///   • +3 dB low shelf at 120 Hz + a +6 dB resonant hump at 115 Hz (cab depth)
///   • +5 dB wide mound at 210 Hz and +3 dB at 480 Hz (low-mid body plateau)
///   • +1 dB at 800 Hz (a hint of the GB "vintage" honk)
///   • -3.5 dB wide dip at 1500 Hz (the mid "pocket" of a real capture)
///   • +4.5 dB at 2500 Hz and +2.5 dB at 4 kHz (GB presence — warmer/lower
///     than V30's 3.5 kHz, but held through the 3–5 kHz band like a real capture)
///   • -11 dB high shelf at 6000 Hz (softer cone rolloff vs V30)
///   • LP at 8 kHz (fizz cut — GBs are inherently smoother on top)
pub struct MarshallCab {
    inner: BlendedCab,
}

// Gentler, slightly later reflections than the V30 (smoother Greenback cone),
// timed so the early comb notches land in the mid pocket rather than the body
// (see the Mesa texture note); breakup mode lower at ~2.5 kHz. The two low modes
// near 90–115 Hz add a subtle thump ring where the direct sound is strong.
const TEX_L: Texture = Texture {
    predelay: 0,
    reflections: &[
        (0.30, -0.26),
        (0.62, 0.17),
        (1.24, -0.09),
        (3.40, 0.075),
        (7.10, -0.070),
        (12.40, 0.058),
        (15.20, -0.047),
        (17.00, 0.038),
        (20.00, 0.029),
    ],
    modes: &[
        (92.0, 95.0, 0.004),
        (110.0, 85.0, 0.004),
        (2500.0, 5.0, 0.009),
    ],
    scatter: Some(ir::Scatter {
        seed: 21,
        count: 22,
        band: (1800.0, 6400.0),
        t60_ms: (2.0, 5.0),
        gain: 0.016,
    }),
};
const TEX_R: Texture = Texture {
    predelay: 2,
    reflections: &[
        (0.34, -0.23),
        (0.66, 0.18),
        (1.32, -0.08),
        (3.70, 0.072),
        (7.80, -0.066),
        (13.30, 0.055),
        (15.90, -0.044),
        (17.50, 0.036),
        (20.50, 0.027),
    ],
    modes: &[
        (94.0, 97.0, 0.004),
        (114.0, 87.0, 0.004),
        (2600.0, 5.0, 0.009),
    ],
    scatter: Some(ir::Scatter {
        seed: 22,
        count: 22,
        band: (1800.0, 6400.0),
        t60_ms: (2.0, 5.0),
        gain: 0.016,
    }),
};

// Room-mic textures: distance pre-delay + denser late reflections for air.
const ROOM_TEX_L: Texture = Texture {
    predelay: 120,
    reflections: &[
        (2.80, 0.20),
        (5.90, -0.16),
        (9.80, 0.13),
        (13.00, -0.10),
        (15.50, 0.075),
        (18.00, -0.055),
    ],
    modes: &[(72.0, 130.0, 0.006), (170.0, 95.0, 0.005)],
    scatter: None,
};
const ROOM_TEX_R: Texture = Texture {
    predelay: 150,
    reflections: &[
        (3.20, 0.18),
        (6.60, -0.15),
        (11.00, 0.12),
        (13.50, -0.095),
        (15.50, 0.07),
        (17.50, -0.05),
    ],
    modes: &[(76.0, 135.0, 0.006), (180.0, 100.0, 0.005)],
    scatter: None,
};

impl MarshallCab {
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

    /// SM57 close-mic: the bright Greenback voicing (the original skeleton).
    fn voicing_sm57(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            // Resonant rise into a ~115 Hz hump, then the broad 120–600 Hz body
            // plateau over a shallow 800 Hz–2 kHz pocket — the deep-and-juicy
            // shape of a real capture (see the Mesa `voicing_sm57` note).
            Biquad::highpass(sr, 74.0, 1.2),
            Biquad::low_shelf(sr, 120.0, 2.0),
            Biquad::peak_eq(sr, 115.0, 1.1, 6.0),
            Biquad::peak_eq(sr, 210.0, 0.7, 5.0),
            Biquad::peak_eq(sr, 480.0, 0.9, 3.0),
            Biquad::peak_eq(sr, 800.0, 1.5, 2.0),
            Biquad::peak_eq(sr, 1500.0, 0.6, -2.0),
            // Greenback presence: broadened and trimmed (Q 1.8→1.4, +5→+4 dB) for a
            // smoother top — Greenbacks are inherently softer up here than V30s.
            Biquad::peak_eq(sr, 2500.0, 1.0, 5.5),
            Biquad::peak_eq(sr, 4300.0, 1.2, 3.5),
            Biquad::high_shelf(sr, 6600.0, -15.0),
            Biquad::lowpass(sr, 7200.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }

    /// R121 ribbon close-mic: warmer still, softer presence, silky top.
    fn voicing_ribbon(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 72.0, 1.2),
            Biquad::low_shelf(sr, 150.0, 2.0),
            Biquad::peak_eq(sr, 105.0, 1.1, 5.5), // low resonant hump (cab depth)
            Biquad::peak_eq(sr, 200.0, 0.7, 5.0), // broad low-mid body mound
            Biquad::peak_eq(sr, 480.0, 0.9, 2.5),
            Biquad::peak_eq(sr, 800.0, 1.3, 2.5),
            Biquad::peak_eq(sr, 2200.0, 1.4, 2.0), // softer, lower presence
            Biquad::high_shelf(sr, 4200.0, -14.0), // ribbon HF rolloff
            Biquad::lowpass(sr, 6000.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }

    /// Room mic: darker, distance-coloured Greenback.
    fn voicing_room(sr: f32) -> impl FnMut(f32) -> f32 {
        let mut bands = [
            Biquad::highpass(sr, 72.0, 0.8),
            Biquad::low_shelf(sr, 150.0, 2.5),
            Biquad::peak_eq(sr, 350.0, 1.2, -2.0),
            Biquad::peak_eq(sr, 900.0, 1.0, 2.0),
            Biquad::high_shelf(sr, 3800.0, -9.0),
            Biquad::lowpass(sr, 5200.0, 0.707),
        ];
        move |x| bands.iter_mut().fold(x, |acc, b| b.process(acc))
    }
}

impl Cabinet for MarshallCab {
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
        analysis::assert_plausible("marshall close L", sr, &TEX_L);
        analysis::assert_plausible("marshall close R", sr, &TEX_R);
        analysis::assert_plausible("marshall room L", sr, &ROOM_TEX_L);
        analysis::assert_plausible("marshall room R", sr, &ROOM_TEX_R);
        analysis::assert_lr_symmetry("marshall close", &TEX_L, &TEX_R);
        analysis::assert_lr_symmetry("marshall room", &ROOM_TEX_L, &ROOM_TEX_R);
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
        check!("marshall close L", MarshallCab::voicing_sm57, &TEX_L);
        check!("marshall close R", MarshallCab::voicing_sm57, &TEX_R);
        check!("marshall room L", MarshallCab::voicing_room, &ROOM_TEX_L);
        check!("marshall room R", MarshallCab::voicing_room, &ROOM_TEX_R);
    }
}
