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
///   • +4 dB wide mound at 220 Hz and +5.5 dB at 500 Hz (low-mid body plateau,
///     carried up through ~700 Hz)
///   • −5 dB dip at 1250 Hz over a −1 dB shelf from 1.35 kHz (the mid
///     "pocket": real captures hold their 0.5–1 kHz level, dip through the
///     1–2 kHz octave, then recover by 2.5 kHz)
///   • +4 dB wide mound at 2.5 kHz and +5.5 dB at 4.5 kHz (V30 presence, held
///     to ~5 kHz — real captures keep their treble through 2.3–5 kHz, then crash)
///   • −17 dB high shelf at 6800 Hz (speaker cone rolloff)
///   • 4th-order LP at 7 kHz (fizz cut — real captures carry no 8 kHz energy)
pub struct MesaCab {
    inner: BlendedCab,
}

// Close-mic texture, shared by both channels. The references this cab is tuned
// against (real close captures — see `examples/cab_analysis.rs`) are effectively
// mono: L/R correlation 1.00. The old per-channel detuned reflection times and
// modes measured 0.66 — phasey width that smeared the solid centre image a real
// capture has. Only the scatter seeds differ per channel now, leaving a whisper
// of top-end width like a real speaker pair whose breakup patterns never match;
// the room mics below stay a genuinely decorrelated stereo pair.
//
// Early taps (< 4 ms) are the cone-to-grille / panel comb that colours the body —
// timed so their comb notches land in the 800 Hz–2 kHz mid pocket rather than in
// the 400–600 Hz body; the later taps (6–21 ms) are cabinet-edge and near-wall
// reflections that put the speaker in a space and give the note depth and air.
// The two low modes near 100–120 Hz add a subtle thump ring on top of the EQ hump
// (they sit where the direct sound is strong: an additive resonance placed where
// the direct path is weak phase-cancels it just above resonance and carves a
// notch instead of adding depth); their T60s are kept short of the note itself —
// the reference captures are gated by ~40 ms, and a longer synthetic ring reads
// as boxy mud, not depth. The ~3.4 kHz mode is the V30 breakup.
// The 6–21 ms tail is deliberately dense (a tap every ~1.5–2 ms): real
// captures measure a diffuse 10–20 ms window (echo crest ~3–4), and the old
// sparse five-tap tail read ~5 — audibly "a few discrete echoes", not a room.
// Early-tap gains are kept small (~0.1): a ±0.2 tap at 0.62 ms is a *voicing*
// move (a −5 dB null at 800/2400 Hz and +3 dB peaks at 1.6/3.2 kHz — measured
// as exactly those deviations vs the refs), and the macro shape belongs to the
// EQ skeleton; at ~0.1 the combs read as texture, like real captures.
const TEX_REFL: &[(f32, f32)] = &[
    (0.27, -0.13),
    (0.62, 0.11),
    (1.24, -0.095),
    (3.10, 0.085),
    (6.30, -0.075),
    (8.40, 0.068),
    (10.80, -0.062),
    (12.40, 0.056),
    (14.20, -0.050),
    (15.90, 0.045),
    (17.50, -0.040),
    (19.00, 0.034),
    (20.50, -0.030),
];
// The breakup mode stays at texture scale (0.045): at 0.1 the added resonance
// out-shouted the voicing — a measured +5 dB peak at its frequency over a
// −5 dB phase-cancellation notch half an octave below.
const TEX_MODES: &[(f32, f32, f32)] = &[
    (98.0, 55.0, 0.004),
    (118.0, 50.0, 0.004),
    (3400.0, 4.0, 0.045),
];
// Two scatter clusters per channel: the V30 breakup band, plus a mid cluster
// for the fine 0.5–2 kHz reflection ripple real captures measure (~1.5–2.5 dB
// of fine-grained texture; the hand-authored taps alone leave the mids
// statistically airbrushed).
const SCATTER_L: &[ir::Scatter] = &[
    ir::Scatter {
        seed: 11,
        count: 22,
        band: (2200.0, 7000.0),
        t60_ms: (5.0, 12.0),
        gain: 0.014,
    },
    ir::Scatter {
        seed: 13,
        count: 24,
        band: (550.0, 2300.0),
        t60_ms: (6.0, 16.0),
        gain: 0.022,
    },
];
const SCATTER_R: &[ir::Scatter] = &[
    ir::Scatter {
        seed: 12,
        count: 22,
        band: (2200.0, 7000.0),
        t60_ms: (5.0, 12.0),
        gain: 0.014,
    },
    ir::Scatter {
        seed: 14,
        count: 24,
        band: (550.0, 2300.0),
        t60_ms: (6.0, 16.0),
        gain: 0.022,
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
    predelay: 110,
    reflections: &[
        (2.50, 0.22),
        (5.40, -0.18),
        (9.10, 0.14),
        (14.00, -0.11),
        (16.50, 0.08),
        (18.50, -0.06),
    ],
    modes: &[(82.0, 65.0, 0.005), (180.0, 55.0, 0.004)],
    scatter: &[],
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
    modes: &[(86.0, 65.0, 0.005), (190.0, 55.0, 0.004)],
    scatter: &[],
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
            // measure: a steep resonant rise into a big ~120 Hz hump — a shelf
            // alone is flat below its corner; the hump is what reads as "deep" —
            // then a broad +5…+10 dB body plateau from ~120 to ~700 Hz relative
            // to the 1–2 kHz band, which instead carries a wide, shallow pocket.
            // That plateau-vs-pocket tilt, not sub-bass, is what makes a capture
            // sound deep and juicy.
            Biquad::highpass(sr, 72.0, 1.2),
            Biquad::low_shelf(sr, 100.0, 3.0),
            Biquad::peak_eq(sr, 120.0, 1.1, 6.5),
            Biquad::peak_eq(sr, 220.0, 0.7, 4.0),
            // The body plateau is carried up through ~700 Hz (wide +5.5 dB at
            // 500 Hz) and the pocket sits at 1.3 kHz: real captures hold their
            // 0.5–1 kHz level and put the dip in the 1–2 kHz octave. Q 0.95
            // keeps the pocket's skirts off the body below and the presence
            // above.
            Biquad::peak_eq(sr, 500.0, 0.7, 5.5),
            // Mid pocket kept narrow (Q 1.0 at 1250) and the post-pocket tilt
            // gentle: the reference captures dip through 1–2 kHz but recover by
            // 2.5 kHz into the presence mound; the old Q 0.95 dip + −3 dB shelf
            // dug a measured −5…−7 dB hole through 2–2.5 kHz.
            Biquad::peak_eq(sr, 1250.0, 1.0, -5.0),
            Biquad::high_shelf(sr, 1350.0, -1.0),
            // V30 presence: a wide, low-centred mound (2.5 kHz, Q 0.6) — the
            // refs recover from the mid pocket by 2–2.5 kHz into a smooth rise,
            // not a spike at 3.1–3.5 kHz over a 2.5 kHz hole (measured −5…−7 dB
            // there against them with the old higher, narrower peak).
            Biquad::peak_eq(sr, 2500.0, 0.65, 4.0),
            // The SM57-on-V30 edge: real captures hold their level through
            // 4.2–4.8 kHz right up to the cone's crash, so this peak carries the
            // presence out to the rolloff shelf.
            Biquad::peak_eq(sr, 4500.0, 1.1, 5.5),
            // Real V30 captures fall off a cliff above ~7 kHz; the old single
            // 8 kHz pole left a measured +10 dB of fizz at 8 kHz vs the refs.
            // The cascaded pair carries the steepness — the shelf stays at −17
            // so 6.3 kHz doesn't collapse before the cliff.
            Biquad::high_shelf(sr, 6800.0, -17.0),
            Biquad::lowpass(sr, 7000.0, 0.707),
            Biquad::lowpass(sr, 7000.0, 0.707),
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
            Biquad::peak_eq(sr, 1400.0, 0.7, -2.5), // mid pocket
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
            scatter: t.scatter,
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
