use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::{AmpModel, CabModel, Params};

use super::config::{ADD_TILE, AMP_END, AMP_START, KNOBS, MIC_END, MIC_START, PEDALS, pedal_of};

/// A knob is reachable only if it belongs to the amp/mic (always present) or to
/// a pedal currently on the board.
fn knob_visible(knob: usize, board: &[bool]) -> bool {
    match pedal_of(knob) {
        Some(p) => board[p],
        None => true,
    }
}

/// The section start a focus belongs to: `None` (selectors), the amp/mic starts,
/// a pedal start, or the `ADD_TILE` sentinel.
fn section_start_of(focus: Option<usize>) -> Option<usize> {
    match focus {
        None => None,
        Some(i) if i == ADD_TILE => Some(ADD_TILE),
        Some(i) if (AMP_START..AMP_END).contains(&i) => Some(AMP_START),
        Some(i) if (MIC_START..MIC_END).contains(&i) => Some(MIC_START),
        Some(i) => pedal_of(i).map(|p| PEDALS[p].start),
    }
}

/// Tab stops: selectors → amp → mic → on-board pedals → +ADD tile.
fn section_stops(board: &[bool]) -> Vec<Option<usize>> {
    let mut v = vec![None, Some(AMP_START), Some(MIC_START)];
    for (i, p) in PEDALS.iter().enumerate() {
        if board[i] {
            v.push(Some(p.start));
        }
    }
    v.push(Some(ADD_TILE));
    v
}

/// Per-knob stops for ←/→: selectors → every visible knob → +ADD tile.
fn knob_stops(board: &[bool]) -> Vec<Option<usize>> {
    let mut v = vec![None];
    v.extend(
        (0..KNOBS.len())
            .filter(|&k| knob_visible(k, board))
            .map(Some),
    );
    v.push(Some(ADD_TILE));
    v
}

fn cycle(stops: &[Option<usize>], current: Option<usize>, dir: i32) -> Option<usize> {
    let n = stops.len() as i32;
    let cur = stops.iter().position(|&s| s == current).unwrap_or(0) as i32;
    stops[(((cur + dir) % n + n) % n) as usize]
}

pub(super) fn next_section(focus: Option<usize>, board: &[bool]) -> Option<usize> {
    cycle(&section_stops(board), section_start_of(focus), 1)
}

pub(super) fn prev_section(focus: Option<usize>, board: &[bool]) -> Option<usize> {
    cycle(&section_stops(board), section_start_of(focus), -1)
}

pub(super) fn nav_knob(focus: Option<usize>, board: &[bool], dir: i32) -> Option<usize> {
    cycle(&knob_stops(board), focus, dir)
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
    // While an external IR is the active cab, `C` returns to the built-in
    // (simulated) cab rather than cycling the inert model selector. The IR stays
    // loaded, so `X` can re-engage it.
    if params.cab_external_active.load(Relaxed) {
        params.cab_external_active.store(false, Relaxed);
        return;
    }
    let current = CabModel::from_u8(params.cab_model.load(Relaxed));
    params.cab_model.store(current.toggle() as u8, Relaxed);
}

pub(super) fn toggle_pedal(params: &Params, knob_idx: usize) {
    // Amp and mic sections have no on/off toggle, so `pedal_of` returns `None`.
    if let Some(p) = pedal_of(knob_idx) {
        let flag = (PEDALS[p].enabled)(params);
        flag.store(!flag.load(Relaxed), Relaxed);
    }
}

/// Put a pedal on the board and engage it (LED on).
pub(super) fn add_pedal(params: &Params, board: &mut [bool], pedal: usize) {
    board[pedal] = true;
    (PEDALS[pedal].enabled)(params).store(true, Relaxed);
}

/// Take a pedal off the board and bypass it in the DSP chain.
pub(super) fn remove_pedal(params: &Params, board: &mut [bool], pedal: usize) {
    board[pedal] = false;
    (PEDALS[pedal].enabled)(params).store(false, Relaxed);
}
