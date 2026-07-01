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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::{AmpModel, CabModel};

    fn board(on: bool) -> Vec<bool> {
        vec![on; PEDALS.len()]
    }

    fn knob(params: &Params, idx: usize) -> f32 {
        (KNOBS[idx].param)(params).load(Relaxed)
    }

    // ── navigation ────────────────────────────────────────────────────────────

    #[test]
    fn tab_cycles_selectors_amp_mic_addtile_when_board_empty() {
        let board = board(false);
        let mut f = None; // amp/cab selectors
        f = next_section(f, &board);
        assert_eq!(f, Some(AMP_START));
        f = next_section(f, &board);
        assert_eq!(f, Some(MIC_START));
        f = next_section(f, &board);
        assert_eq!(f, Some(ADD_TILE));
        f = next_section(f, &board);
        assert_eq!(f, None, "Tab must wrap back to the selectors");
    }

    #[test]
    fn shift_tab_is_the_inverse_of_tab() {
        let board = board(true);
        let mut f = None;
        // Walk forward through every stop, then back, and confirm we retrace it.
        let mut forward = vec![f];
        for _ in 0..PEDALS.len() + 4 {
            f = next_section(f, &board);
            forward.push(f);
        }
        for &expected in forward.iter().rev().skip(1) {
            f = prev_section(f, &board);
            assert_eq!(f, expected, "BackTab did not retrace Tab");
        }
    }

    #[test]
    fn tab_visits_only_on_board_pedals() {
        let mut b = board(false);
        b[3] = true; // only the DELAY pedal is on the board
        let stops = section_stops(&b);
        assert!(stops.contains(&Some(PEDALS[3].start)));
        for (i, p) in PEDALS.iter().enumerate() {
            if i != 3 {
                assert!(
                    !stops.contains(&Some(p.start)),
                    "{} is off the board but appears as a Tab stop",
                    p.name
                );
            }
        }
    }

    #[test]
    fn arrow_nav_skips_knobs_of_off_board_pedals() {
        let b = board(false); // only amp + mic knobs are visible
        let stops = knob_stops(&b);
        assert!(
            !stops.iter().flatten().any(|&k| pedal_of(k).is_some()),
            "a hidden pedal's knob is reachable with ←/→"
        );
        // Every amp/mic knob is still reachable.
        for k in AMP_START..MIC_END {
            assert!(
                stops.contains(&Some(k)),
                "amp/mic knob {k} is not reachable"
            );
        }
    }

    #[test]
    fn arrow_nav_wraps_at_both_ends() {
        let b = board(false);
        // From the selectors, ← wraps to the last stop (the +ADD tile).
        assert_eq!(nav_knob(None, &b, -1), Some(ADD_TILE));
        // From the +ADD tile, → wraps back to the selectors.
        assert_eq!(nav_knob(Some(ADD_TILE), &b, 1), None);
    }

    // ── knob edits ──────────────────────────────────────────────────────────────

    #[test]
    fn nudge_clamps_to_the_unit_range() {
        let p = Params::new();
        nudge(&p, 0, 5.0);
        assert_eq!(knob(&p, 0), 1.0, "nudge above 1.0 must clamp");
        nudge(&p, 0, -5.0);
        assert_eq!(knob(&p, 0), 0.0, "nudge below 0.0 must clamp");
    }

    #[test]
    fn nudge_moves_only_the_targeted_knob() {
        let p = Params::new();
        let before: Vec<f32> = (0..KNOBS.len()).map(|k| knob(&p, k)).collect();
        nudge(&p, 2, 0.05);
        for (k, knob_before) in before.iter().enumerate().take(KNOBS.len()) {
            if k == 2 {
                assert!((knob(&p, k) - (knob_before + 0.05)).abs() < 1e-6);
            } else {
                assert_eq!(knob(&p, k), *knob_before, "knob {k} moved unexpectedly");
            }
        }
    }

    // ── amp / cab selectors ─────────────────────────────────────────────────────

    #[test]
    fn cycle_amp_forward_visits_every_model_and_returns() {
        let p = Params::new();
        p.amp_model.store(AmpModel::Marshall as u8, Relaxed);
        cycle_amp(&p, 1);
        assert_eq!(AmpModel::from_u8(p.amp_model.load(Relaxed)), AmpModel::Mesa);
        cycle_amp(&p, 1);
        assert_eq!(
            AmpModel::from_u8(p.amp_model.load(Relaxed)),
            AmpModel::Randall
        );
        cycle_amp(&p, 1);
        assert_eq!(AmpModel::from_u8(p.amp_model.load(Relaxed)), AmpModel::Vox);
        cycle_amp(&p, 1);
        assert_eq!(
            AmpModel::from_u8(p.amp_model.load(Relaxed)),
            AmpModel::Marshall,
            "amp selector must cycle back to the start"
        );
    }

    #[test]
    fn cycle_amp_backward_is_the_inverse() {
        let p = Params::new();
        p.amp_model.store(AmpModel::Marshall as u8, Relaxed);
        cycle_amp(&p, -1);
        assert_eq!(AmpModel::from_u8(p.amp_model.load(Relaxed)), AmpModel::Vox);
    }

    #[test]
    fn cycle_cab_toggles_through_every_built_in_model() {
        let p = Params::new();
        p.cab_external_active.store(false, Relaxed);
        p.cab_model.store(CabModel::Mesa as u8, Relaxed);
        cycle_cab(&p);
        assert_eq!(
            CabModel::from_u8(p.cab_model.load(Relaxed)),
            CabModel::Marshall
        );
        cycle_cab(&p);
        assert_eq!(
            CabModel::from_u8(p.cab_model.load(Relaxed)),
            CabModel::Orange
        );
        cycle_cab(&p);
        assert_eq!(CabModel::from_u8(p.cab_model.load(Relaxed)), CabModel::Mesa);
    }

    #[test]
    fn cycle_cab_returns_to_built_in_when_external_ir_is_active() {
        let p = Params::new();
        p.cab_external_active.store(true, Relaxed);
        let model_before = p.cab_model.load(Relaxed);
        cycle_cab(&p);
        // First press only deactivates the external IR; it leaves the built-in
        // model selector untouched so `X` can re-engage the same IR.
        assert!(!p.cab_external_active.load(Relaxed));
        assert_eq!(p.cab_model.load(Relaxed), model_before);
    }

    // ── board membership & toggles ──────────────────────────────────────────────

    #[test]
    fn add_then_remove_pedal_round_trips_board_and_enabled_flag() {
        let p = Params::new();
        let mut b = board(false);
        // Force the flag off first so the default-on pedals don't mask the test.
        (PEDALS[5].enabled)(&p).store(false, Relaxed);

        add_pedal(&p, &mut b, 5);
        assert!(b[5], "add_pedal must put the pedal on the board");
        assert!(
            (PEDALS[5].enabled)(&p).load(Relaxed),
            "add_pedal must engage the LED"
        );

        remove_pedal(&p, &mut b, 5);
        assert!(!b[5], "remove_pedal must take it off the board");
        assert!(
            !(PEDALS[5].enabled)(&p).load(Relaxed),
            "remove_pedal must bypass it"
        );
    }

    #[test]
    fn toggle_pedal_flips_the_enabled_flag_for_a_pedal_knob() {
        let p = Params::new();
        let flag = (PEDALS[0].enabled)(&p);
        let before = flag.load(Relaxed);
        toggle_pedal(&p, PEDALS[0].start);
        assert_eq!(flag.load(Relaxed), !before);
    }

    #[test]
    fn toggle_pedal_is_a_noop_for_amp_and_mic_knobs() {
        let p = Params::new();
        // Amp/mic knobs own no enable flag; toggling must not panic or flip anything.
        let flags_before: Vec<bool> = PEDALS
            .iter()
            .map(|pd| (pd.enabled)(&p).load(Relaxed))
            .collect();
        toggle_pedal(&p, AMP_START);
        toggle_pedal(&p, MIC_START);
        let flags_after: Vec<bool> = PEDALS
            .iter()
            .map(|pd| (pd.enabled)(&p).load(Relaxed))
            .collect();
        assert_eq!(flags_before, flags_after);
    }
}
