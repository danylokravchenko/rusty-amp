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
        |_| "⚡ TS-808".into(),
        0,
        3,
        |p| Some(p.ts_enabled.load(Relaxed)),
    ),
    (
        |_| "⚡ DS-1 DISTORTION".into(),
        3,
        6,
        |p| Some(p.ds_enabled.load(Relaxed)),
    ),
    (
        |_| "⚡ SPRING REVERB".into(),
        6,
        9,
        |p| Some(p.rev_enabled.load(Relaxed)),
    ),
];

pub(super) const AMP_SECTIONS: &[SectionDef] = &[(|p| format!("⚡ {}", p.amp_model().name()), 9, 14, |_| None)];

pub(super) const SECTION_STARTS: &[Option<usize>] = &[None, Some(0), Some(3), Some(6), Some(9)];
