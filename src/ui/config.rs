use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use atomic_float::AtomicF32;
use ratatui::style::Color;

use super::styles::{
    PEDAL_BLUE, PEDAL_GOLD, PEDAL_GREEN, PEDAL_INDIGO, PEDAL_LIME, PEDAL_ORANGE, PEDAL_PURPLE,
    PEDAL_RED, PEDAL_SILVER, PEDAL_TEAL,
};
use crate::dsp::Params;

pub(super) struct Knob {
    pub(super) label: &'static str,
    pub(super) param: fn(&Params) -> &Arc<AtomicF32>,
}

/// A rig pedal: its livery, the slice of `KNOBS` it owns, and its on/off flag.
/// `render_rig` walks this table to draw both the compact tiles and the detail
/// editor, so adding a pedal is a single entry here (plus its knobs above).
pub(super) struct Pedal {
    pub(super) name: &'static str,
    pub(super) color: Color,
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) enabled: fn(&Params) -> &Arc<AtomicBool>,
}

// Knob-index ranges: each section owns a contiguous `[START, END)` slice of the
// KNOBS array below. The amp/mic panels and the PEDALS table reference these
// bounds.
//
// IMPORTANT: ←/→ navigation walks KNOBS linearly, so this order — amp tone
// stack, cabinet mics, then the pedals in signal-chain order — must match the
// KNOBS array one-to-one.
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
pub(super) const FL_START: usize = 35;
pub(super) const FL_END: usize = 39;

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
    // 35–38: Flanger
    Knob {
        label: "RATE",
        param: |p| &p.fl_rate,
    },
    Knob {
        label: "DEPTH",
        param: |p| &p.fl_depth,
    },
    Knob {
        label: "FEEDBACK",
        param: |p| &p.fl_feedback,
    },
    Knob {
        label: "MIX",
        param: |p| &p.fl_mix,
    },
];

// Rig pedals in navigation order (mirrors the KNOBS slices above). The tile
// grid and detail editor both iterate this table.
pub(super) const PEDALS: &[Pedal] = &[
    Pedal {
        name: "TS-808",
        color: PEDAL_GREEN,
        start: TS_START,
        end: TS_END,
        enabled: |p| &p.ts_enabled,
    },
    Pedal {
        name: "DS-1",
        color: PEDAL_ORANGE,
        start: DS_START,
        end: DS_END,
        enabled: |p| &p.ds_enabled,
    },
    Pedal {
        name: "SPRING REVERB",
        color: PEDAL_BLUE,
        start: REV_START,
        end: REV_END,
        enabled: |p| &p.rev_enabled,
    },
    Pedal {
        name: "DELAY",
        color: PEDAL_PURPLE,
        start: DELAY_START,
        end: DELAY_END,
        enabled: |p| &p.delay_enabled,
    },
    Pedal {
        name: "COMP",
        color: PEDAL_GOLD,
        start: CMP_START,
        end: CMP_END,
        enabled: |p| &p.cmp_enabled,
    },
    Pedal {
        name: "FUZZ",
        color: PEDAL_RED,
        start: FUZZ_START,
        end: FUZZ_END,
        enabled: |p| &p.fz_enabled,
    },
    Pedal {
        name: "NOISE GATE",
        color: PEDAL_SILVER,
        start: NG_START,
        end: NG_END,
        enabled: |p| &p.ng_enabled,
    },
    Pedal {
        name: "PRE-AMP EQ",
        color: PEDAL_LIME,
        start: PEQ_START,
        end: PEQ_END,
        enabled: |p| &p.peq_enabled,
    },
    Pedal {
        name: "PARAMETRIC EQ",
        color: PEDAL_TEAL,
        start: EQ_START,
        end: EQ_END,
        enabled: |p| &p.eq_enabled,
    },
    Pedal {
        name: "FLANGER",
        color: PEDAL_INDIGO,
        start: FL_START,
        end: FL_END,
        enabled: |p| &p.fl_enabled,
    },
];

// Sentinel focus value for the "+ ADD" tile at the end of the board. It is not
// a real knob index, so any code that indexes `KNOBS` must guard against it.
pub(super) const ADD_TILE: usize = KNOBS.len();

/// Index into `PEDALS` owning the given knob, or `None` for amp/mic knobs.
pub(super) fn pedal_of(knob: usize) -> Option<usize> {
    PEDALS.iter().position(|p| (p.start..p.end).contains(&knob))
}
