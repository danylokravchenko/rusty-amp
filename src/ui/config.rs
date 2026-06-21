use std::sync::Arc;

use atomic_float::AtomicF32;

use crate::dsp::Params;

pub(super) struct Knob {
    pub(super) label: &'static str,
    pub(super) param: fn(&Params) -> &Arc<AtomicF32>,
}

// Knob index range boundaries.
//
// Order mirrors the on-screen layout top-to-bottom, left-to-right: the
// amplifier head (tone stack + mic) sits above the pedalboard rig, so its
// knobs come first, then the rig pedals in rows.
pub(super) const AMP_START: usize = 0;
pub(super) const AMP_END: usize = 6;
pub(super) const MIC_START: usize = 6;
pub(super) const MIC_END: usize = 7;
pub(super) const TS_START: usize = 7;
pub(super) const TS_END: usize = 10;
pub(super) const DS_START: usize = 10;
pub(super) const DS_END: usize = 13;
pub(super) const REV_START: usize = 13;
pub(super) const REV_END: usize = 16;
pub(super) const DELAY_START: usize = 16;
pub(super) const DELAY_END: usize = 19;
pub(super) const NG_START: usize = 19;
pub(super) const NG_END: usize = 21;
pub(super) const EQ_START: usize = 21;
pub(super) const EQ_END: usize = 24;

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
    // 6: Mic position
    Knob {
        label: "MIC POS",
        param: |p| &p.mic_pos,
    },
    // 7–9: TS-808
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
    // 10–12: DS-1
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
    // 13–15: Reverb
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
    // 16–18: Delay
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
    // 19–20: Noise Gate
    Knob {
        label: "THRESH",
        param: |p| &p.ng_threshold,
    },
    Knob {
        label: "RELEASE",
        param: |p| &p.ng_release,
    },
    // 21–23: Parametric EQ
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

// Tab order follows the layout: selectors → Amp → Mic → rig pedals.
pub(super) const SECTION_STARTS: &[Option<usize>] = &[
    None,
    Some(AMP_START),
    Some(MIC_START),
    Some(TS_START),
    Some(DS_START),
    Some(REV_START),
    Some(DELAY_START),
    Some(NG_START),
    Some(EQ_START),
];
