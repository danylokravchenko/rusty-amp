use std::sync::Arc;

use atomic_float::AtomicF32;
use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::Params;

pub(super) struct Knob {
    pub(super) label: &'static str,
    pub(super) param: fn(&Params) -> &Arc<AtomicF32>,
}

pub(super) type SectionDef = (
    fn(&Params) -> String,
    usize,
    usize,
    fn(&Params) -> Option<bool>,
);

pub(super) const KNOBS: &[Knob] = &[
    // 0–1: Noise Gate
    Knob {
        label: "THRESH",
        param: |p| &p.ng_threshold,
    },
    Knob {
        label: "RELEASE",
        param: |p| &p.ng_release,
    },
    // 2–4: TS-808
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
    // 5–7: DS-1
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
    // 8–10: Reverb
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
    // 11–13: Parametric EQ
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
    // 14–16: Delay
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
    // 17–21: Amp
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
        label: "MASTER",
        param: |p| &p.amp_master,
    },
];

pub(super) const PEDAL_SECTIONS: &[SectionDef] = &[
    (
        |_| "🔇 NOISE GATE".into(),
        0,
        2,
        |p| Some(p.ng_enabled.load(Relaxed)),
    ),
    (
        |_| "⚡ TS-808".into(),
        2,
        5,
        |p| Some(p.ts_enabled.load(Relaxed)),
    ),
    (
        |_| "⚡ DS-1 DISTORTION".into(),
        5,
        8,
        |p| Some(p.ds_enabled.load(Relaxed)),
    ),
    (
        |_| "⚡ SPRING REVERB".into(),
        8,
        11,
        |p| Some(p.rev_enabled.load(Relaxed)),
    ),
];

pub(super) const AMP_SECTIONS: &[SectionDef] = &[
    (|p| format!("⚡ {}", p.amp_model().name()), 17, 22, |_| None),
    (
        |_| "🎛 PARAMETRIC EQ".into(),
        11,
        14,
        |p| Some(p.eq_enabled.load(Relaxed)),
    ),
    (
        |_| "⏱ DELAY".into(),
        14,
        17,
        |p| Some(p.delay_enabled.load(Relaxed)),
    ),
];

// Tab order: None (selectors) → NG → TS → DS → Rev → EQ → Delay → Amp
pub(super) const SECTION_STARTS: &[Option<usize>] = &[
    None,
    Some(0),
    Some(2),
    Some(5),
    Some(8),
    Some(11),
    Some(14),
    Some(17),
];
