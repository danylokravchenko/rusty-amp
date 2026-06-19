use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::{AmpModel, CabModel, Params};

use super::config::{KNOBS, SECTION_STARTS};

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
    if knob_idx < 2 {
        let v = params.ng_enabled.load(Relaxed);
        params.ng_enabled.store(!v, Relaxed);
    } else if knob_idx < 5 {
        let v = params.ts_enabled.load(Relaxed);
        params.ts_enabled.store(!v, Relaxed);
    } else if knob_idx < 8 {
        let v = params.ds_enabled.load(Relaxed);
        params.ds_enabled.store(!v, Relaxed);
    } else if knob_idx < 11 {
        let v = params.rev_enabled.load(Relaxed);
        params.rev_enabled.store(!v, Relaxed);
    } else if knob_idx < 14 {
        let v = params.eq_enabled.load(Relaxed);
        params.eq_enabled.store(!v, Relaxed);
    } else if knob_idx < 17 {
        let v = params.delay_enabled.load(Relaxed);
        params.delay_enabled.store(!v, Relaxed);
    }
}

fn section_of(focus: Option<usize>) -> usize {
    match focus {
        None => 0,
        Some(i) if i < 2 => 1,
        Some(i) if i < 5 => 2,
        Some(i) if i < 8 => 3,
        Some(i) if i < 11 => 4,
        Some(i) if i < 14 => 5,
        Some(i) if i < 17 => 6,
        Some(_) => 7,
    }
}
