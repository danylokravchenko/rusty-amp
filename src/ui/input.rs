use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::{AmpModel, CabModel, Params};

use super::config::{
    AMP_END, DELAY_END, DS_END, EQ_END, KNOBS, MIC_END, NG_END, REV_END, SECTION_STARTS, TS_END,
};

pub(super) fn next_section(focus: Option<usize>) -> Option<usize> {
    let next = (section_of(focus) + 1) % SECTION_STARTS.len();
    SECTION_STARTS[next]
}

pub(super) fn prev_section(focus: Option<usize>) -> Option<usize> {
    let cur = section_of(focus);
    let prev = (cur + SECTION_STARTS.len() - 1) % SECTION_STARTS.len();
    SECTION_STARTS[prev]
}

pub(super) fn nudge(params: &Params, idx: usize, delta: f32) {
    let atom = (KNOBS[idx].param)(params);
    let new = (atom.load(Relaxed) + delta).clamp(0.0, 1.0);
    atom.store(new, Relaxed);
}

pub(super) fn cycle_amp(params: &Params, dir: i8) {
    let current = AmpModel::from_u8(params.amp_model.load(Relaxed));
    let next = if dir >= 0 {
        current.next()
    } else {
        current.prev()
    };
    params.amp_model.store(next as u8, Relaxed);
}

pub(super) fn cycle_cab(params: &Params) {
    let current = CabModel::from_u8(params.cab_model.load(Relaxed));
    params.cab_model.store(current.toggle() as u8, Relaxed);
}

pub(super) fn toggle_pedal(params: &Params, knob_idx: usize) {
    if knob_idx < TS_END {
        let v = params.ts_enabled.load(Relaxed);
        params.ts_enabled.store(!v, Relaxed);
    } else if knob_idx < DS_END {
        let v = params.ds_enabled.load(Relaxed);
        params.ds_enabled.store(!v, Relaxed);
    } else if knob_idx < REV_END {
        let v = params.rev_enabled.load(Relaxed);
        params.rev_enabled.store(!v, Relaxed);
    } else if knob_idx < DELAY_END {
        let v = params.delay_enabled.load(Relaxed);
        params.delay_enabled.store(!v, Relaxed);
    } else if knob_idx < NG_END {
        let v = params.ng_enabled.load(Relaxed);
        params.ng_enabled.store(!v, Relaxed);
    } else if knob_idx < AMP_END {
        // Amp has no toggle
    } else if knob_idx < EQ_END {
        let v = params.eq_enabled.load(Relaxed);
        params.eq_enabled.store(!v, Relaxed);
    }
    // MIC section has no toggle
}

fn section_of(focus: Option<usize>) -> usize {
    // Matches SECTION_STARTS order: None, TS, DS, Rev, Delay, NG, Amp, EQ, Mic
    match focus {
        None => 0,
        Some(i) if i < TS_END => 1,
        Some(i) if i < DS_END => 2,
        Some(i) if i < REV_END => 3,
        Some(i) if i < DELAY_END => 4,
        Some(i) if i < NG_END => 5,
        Some(i) if i < AMP_END => 6,
        Some(i) if i < EQ_END => 7,
        Some(i) if i < MIC_END => 8,
        Some(_) => 8,
    }
}
