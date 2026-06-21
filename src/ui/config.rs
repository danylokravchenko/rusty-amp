use std::sync::Arc;

use atomic_float::AtomicF32;

use crate::dsp::Params;

pub(super) struct Knob {
    pub(super) label: &'static str,
    pub(super) param: fn(&Params) -> &Arc<AtomicF32>,
}

// Knob index range boundaries.
//
// IMPORTANT: this order must mirror the on-screen layout top-to-bottom,
// left-to-right, because Left/Right navigation walks the KNOBS array by ±1.
// The amplifier head (tone stack + mic) sits above the pedalboard rig, then
// rig row 1 (TS, DS, Reverb, Delay), then rig row 2 (Comp, Fuzz, Noise Gate,
// Pre-amp EQ, Parametric EQ). The KNOBS array below is in the same order.
pub(super) const AMP_START: usize = 0;
pub(super) const AMP_END: usize = 6;
pub(super) const MIC_START: usize = 6;
pub(super) const MIC_END: usize = 9;
pub(super) const TS_START: usize = 9;
pub(super) const TS_END: usize = 12;
pub(super) const DS_START: usize = 12;
pub(super) const DS_END: usize = 15;
pub(super) const REV_START: usize = 15;
pub(super) const REV_END: usize = 18;
pub(super) const DELAY_START: usize = 18;
pub(super) const DELAY_END: usize = 21;
pub(super) const CMP_START: usize = 21;
pub(super) const CMP_END: usize = 24;
pub(super) const FUZZ_START: usize = 24;
pub(super) const FUZZ_END: usize = 27;
pub(super) const NG_START: usize = 27;
pub(super) const NG_END: usize = 29;
pub(super) const PEQ_START: usize = 29;
pub(super) const PEQ_END: usize = 32;
pub(super) const EQ_START: usize = 32;
pub(super) const EQ_END: usize = 35;

pub(super) const KNOBS: &[Knob] = &[
    // 0–5: Amp tone stack
    Knob {
        label: "GAIN",
        param: |p| &p.amp_gain,
    },
    Knob {
        label: "BASS",
        param: |p| &p.amp_bass,
    },
    Knob {
        label: "MID",
        param: |p| &p.amp_mid,
    },
    Knob {
        label: "TREBLE",
        param: |p| &p.amp_treble,
    },
    Knob {
        label: "PRESENCE",
        param: |p| &p.amp_presence,
    },
    Knob {
        label: "MASTER",
        param: |p| &p.amp_master,
    },
    // 6–8: Cabinet mics (position, dynamic↔ribbon blend, room amount)
    Knob {
        label: "MIC",
        param: |p| &p.mic_pos,
    },
    Knob {
        label: "BLEND",
        param: |p| &p.mic_blend,
    },
    Knob {
        label: "ROOM",
        param: |p| &p.mic_room,
    },
    // 9–11: TS-808
    Knob {
        label: "DRIVE",
        param: |p| &p.ts_drive,
    },
    Knob {
        label: "TONE",
        param: |p| &p.ts_tone,
    },
    Knob {
        label: "LEVEL",
        param: |p| &p.ts_level,
    },
    // 12–14: DS-1
    Knob {
        label: "DRIVE",
        param: |p| &p.ds_drive,
    },
    Knob {
        label: "TONE",
        param: |p| &p.ds_tone,
    },
    Knob {
        label: "LEVEL",
        param: |p| &p.ds_level,
    },
    // 15–17: Reverb
    Knob {
        label: "ROOM",
        param: |p| &p.rev_room,
    },
    Knob {
        label: "DAMP",
        param: |p| &p.rev_damp,
    },
    Knob {
        label: "MIX",
        param: |p| &p.rev_mix,
    },
    // 18–20: Delay
    Knob {
        label: "TIME",
        param: |p| &p.delay_time,
    },
    Knob {
        label: "FEEDBACK",
        param: |p| &p.delay_feedback,
    },
    Knob {
        label: "MIX",
        param: |p| &p.delay_mix,
    },
    // 21–23: Compressor (rig row 2, first)
    Knob {
        label: "SUSTAIN",
        param: |p| &p.cmp_sustain,
    },
    Knob {
        label: "ATTACK",
        param: |p| &p.cmp_attack,
    },
    Knob {
        label: "LEVEL",
        param: |p| &p.cmp_level,
    },
    // 24–26: Fuzz
    Knob {
        label: "FUZZ",
        param: |p| &p.fz_fuzz,
    },
    Knob {
        label: "TONE",
        param: |p| &p.fz_tone,
    },
    Knob {
        label: "LEVEL",
        param: |p| &p.fz_level,
    },
    // 27–28: Noise Gate
    Knob {
        label: "THRESH",
        param: |p| &p.ng_threshold,
    },
    Knob {
        label: "RELEASE",
        param: |p| &p.ng_release,
    },
    // 29–31: Pre-amp EQ
    Knob {
        label: "LOW",
        param: |p| &p.peq_low,
    },
    Knob {
        label: "MID",
        param: |p| &p.peq_mid,
    },
    Knob {
        label: "HIGH",
        param: |p| &p.peq_high,
    },
    // 32–34: Parametric EQ
    Knob {
        label: "LOW",
        param: |p| &p.eq_low,
    },
    Knob {
        label: "MID",
        param: |p| &p.eq_mid,
    },
    Knob {
        label: "HIGH",
        param: |p| &p.eq_high,
    },
];

// Tab order follows the layout: selectors → Amp → Mic → rig pedals
// (row 1 left-to-right, then row 2 left-to-right).
pub(super) const SECTION_STARTS: &[Option<usize>] = &[
    None,
    Some(AMP_START),
    Some(MIC_START),
    Some(TS_START),
    Some(DS_START),
    Some(REV_START),
    Some(DELAY_START),
    Some(CMP_START),
    Some(FUZZ_START),
    Some(NG_START),
    Some(PEQ_START),
    Some(EQ_START),
];
