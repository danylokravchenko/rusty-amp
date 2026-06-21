use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::{AmpModel, CabModel, Params};

use super::config::{
    AMP_END, AMP_START, DELAY_END, DELAY_START, DS_END, DS_START, EQ_END, EQ_START, FUZZ_END,
    FUZZ_START, KNOBS, MIC_END, MIC_START, NG_END, NG_START, REV_END, REV_START, SECTION_STARTS,
    TS_END, TS_START,
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
    // Amp and mic sections have no on/off toggle.
    let flag = if (FUZZ_START..FUZZ_END).contains(&knob_idx) {
        &params.fz_enabled
    } else if (TS_START..TS_END).contains(&knob_idx) {
        &params.ts_enabled
    } else if (DS_START..DS_END).contains(&knob_idx) {
        &params.ds_enabled
    } else if (REV_START..REV_END).contains(&knob_idx) {
        &params.rev_enabled
    } else if (DELAY_START..DELAY_END).contains(&knob_idx) {
        &params.delay_enabled
    } else if (NG_START..NG_END).contains(&knob_idx) {
        &params.ng_enabled
    } else if (EQ_START..EQ_END).contains(&knob_idx) {
        &params.eq_enabled
    } else {
        return;
    };
    let v = flag.load(Relaxed);
    flag.store(!v, Relaxed);
}

fn section_of(focus: Option<usize>) -> usize {
    // Matches SECTION_STARTS order: None, Amp, Mic, TS, DS, Rev, Delay, Fuzz, NG, EQ
    match focus {
        None => 0,
        Some(i) if (AMP_START..AMP_END).contains(&i) => 1,
        Some(i) if (MIC_START..MIC_END).contains(&i) => 2,
        Some(i) if (TS_START..TS_END).contains(&i) => 3,
        Some(i) if (DS_START..DS_END).contains(&i) => 4,
        Some(i) if (REV_START..REV_END).contains(&i) => 5,
        Some(i) if (DELAY_START..DELAY_END).contains(&i) => 6,
        Some(i) if (FUZZ_START..FUZZ_END).contains(&i) => 7,
        Some(i) if (NG_START..NG_END).contains(&i) => 8,
        _ => 9,
    }
}
