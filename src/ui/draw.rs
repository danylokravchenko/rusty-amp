use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::{AmpModel, CabModel, Levels, Params};

use super::config::{ADD_TILE, AMP_END, AMP_START, KNOBS, MIC_END, MIC_START, PEDALS, Pedal};
use super::styles::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    f: &mut Frame,
    params: &Params,
    levels: &Levels,
    focus: Option<usize>,
    board: &[bool],
    recording: bool,
    blink: bool,
    status: Option<&str>,
    plugin: Option<&str>,
    ext_cab: Option<&str>,
    ext_amp: Option<&str>,
) {
    let area = f.area();

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(WARM))
        .style(Style::default().bg(Color::Black));
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(3), // meters
            Constraint::Length(3), // amp / cab selector
            Constraint::Length(7), // amplifier + cabinet/mic
            Constraint::Min(0),    // guitar rig
            Constraint::Length(1), // help
        ])
        .split(inner);

    render_header(
        f, rows[0], params, recording, blink, plugin, ext_cab, ext_amp,
    );
    render_meters(f, rows[1], levels);
    render_amp_selector(f, rows[2], params, focus.is_none());
    render_amp(f, rows[3], params, focus, ext_cab, ext_amp);
    render_rig(f, rows[4], params, board, focus);
    render_help(f, rows[5], status);
}

#[allow(clippy::too_many_arguments)]
fn render_header(
    f: &mut Frame,
    area: Rect,
    params: &Params,
    recording: bool,
    blink: bool,
    plugin: Option<&str>,
    ext_cab: Option<&str>,
    ext_amp: Option<&str>,
) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(WARM))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    // An active external amp (hosted AU) replaces the built-in amp label in the header.
    let amp_name = match ext_amp {
        Some(name) => format!("AU: {}", name.to_uppercase()),
        None => params.amp_model().name().to_uppercase(),
    };
    // An active external IR replaces the built-in cab label in the header.
    let cab_name = match ext_cab {
        Some(name) => format!("IR: {}", name.to_uppercase()),
        None => params.cab_model().name().to_uppercase(),
    };

    let mut title_spans = vec![
        Span::styled(
            "  R U S T Y  A M P  ",
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            concat!("v", env!("CARGO_PKG_VERSION"), "  "),
            Style::default().fg(DIM),
        ),
        Span::styled("▐", Style::default().fg(WARM)),
        Span::styled(
            format!("  {amp_name}  "),
            Style::default().fg(CHROME).add_modifier(Modifier::BOLD),
        ),
        Span::styled("▐", Style::default().fg(WARM)),
        Span::styled(
            format!("  {cab_name}  "),
            Style::default().fg(CHROME).add_modifier(Modifier::BOLD),
        ),
        Span::styled("▐", Style::default().fg(WARM)),
    ];

    // Loaded plugin insert (if any), between the cabinet and the power lamp.
    if let Some(name) = plugin {
        title_spans.push(Span::styled(
            format!("  🔌 {name}  "),
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled("▐", Style::default().fg(WARM)));
    }

    title_spans.push(Span::styled(
        "  ● POWER ON  ",
        Style::default().fg(SAFE).add_modifier(Modifier::BOLD),
    ));
    title_spans.push(Span::styled("▐", Style::default().fg(WARM)));
    title_spans.push(if recording && blink {
        Span::styled(
            "  ● ON AIR  ",
            Style::default().fg(HOT).add_modifier(Modifier::BOLD),
        )
    } else if recording {
        Span::styled("  ○ ON AIR  ", Style::default().fg(HOT))
    } else {
        Span::styled("  ○ OFF AIR  ", Style::default().fg(OFF))
    });

    f.render_widget(Paragraph::new(Line::from(title_spans)), rows[0]);

    let ng_on = params.ng_enabled.load(Relaxed);
    let cmp_on = params.cmp_enabled.load(Relaxed);
    let fz_on = params.fz_enabled.load(Relaxed);
    let ts_on = params.ts_enabled.load(Relaxed);
    let ds_on = params.ds_enabled.load(Relaxed);
    let peq_on = params.peq_enabled.load(Relaxed);
    let eq_on = params.eq_enabled.load(Relaxed);
    let fl_on = params.fl_enabled.load(Relaxed);
    let ch_on = params.ch_enabled.load(Relaxed);
    let delay_on = params.delay_enabled.load(Relaxed);
    let rev_on = params.rev_enabled.load(Relaxed);

    let arrow = Span::styled(" ──▶ ", Style::default().fg(DIM));

    // The ribbon shows the live signal path: only pedals that are on appear, so it
    // mirrors what is actually being heard. AMP and CAB are always in the path and
    // anchor the pre-amp pedals to their post-cab counterparts.
    let pre_pedals = [
        ("GATE", ng_on),
        ("COMP", cmp_on),
        ("FUZZ", fz_on),
        ("TS-808", ts_on),
        ("DS-1", ds_on),
        ("PRE-EQ", peq_on),
    ];
    let post_pedals = [
        ("EQ", eq_on),
        ("FLANGER", fl_on),
        ("CHORUS", ch_on),
        ("DELAY", delay_on),
        ("REVERB", rev_on),
    ];

    let mut chain: Vec<Span> = vec![Span::raw("  ")];
    let push_stage = |chain: &mut Vec<Span>, label: &'static str, color: Color| {
        if chain.len() > 1 {
            chain.push(arrow.clone());
        }
        chain.push(Span::styled(label, Style::default().fg(color)));
    };

    for (label, on) in pre_pedals {
        if on {
            push_stage(&mut chain, label, ORANGE);
        }
    }
    push_stage(&mut chain, "AMP", AMBER);
    push_stage(&mut chain, "CAB", AMBER);
    for (label, on) in post_pedals {
        if on {
            push_stage(&mut chain, label, ORANGE);
        }
    }
    push_stage(&mut chain, "OUTPUT", CHROME);

    f.render_widget(Paragraph::new(Line::from(chain)), rows[1]);
}

fn render_meters(f: &mut Frame, area: Rect, levels: &Levels) {
    let block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let in_level = levels.input.load(Relaxed);
    let out_level = levels.output.load(Relaxed);
    let out_db = amp_to_db(out_level);
    let watts = (out_level * out_level * 100.0).min(100.0);

    render_vu_row(f, rows[0], "  INPUT  ", in_level);
    render_vu_row(f, rows[1], " OUTPUT  ", out_level);

    let scale_line = Paragraph::new(Line::from(vec![
        Span::styled("         ", Style::default()),
        Span::styled("-60", Style::default().fg(DIM)),
        Span::styled("        ", Style::default()),
        Span::styled("-48", Style::default().fg(DIM)),
        Span::styled("        ", Style::default()),
        Span::styled("-36", Style::default().fg(DIM)),
        Span::styled("        ", Style::default()),
        Span::styled("-24", Style::default().fg(DIM)),
        Span::styled("        ", Style::default()),
        Span::styled("-12", Style::default().fg(DIM)),
        Span::styled("       ", Style::default()),
        Span::styled("-6", Style::default().fg(DIM)),
        Span::styled("     ", Style::default()),
        Span::styled("0 dB", Style::default().fg(CHROME)),
        Span::styled("    ", Style::default()),
        Span::styled(
            format!("{:.0}W", watts),
            Style::default()
                .fg(if watts > 80.0 {
                    HOT
                } else if watts > 40.0 {
                    WARN
                } else {
                    SAFE
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {:>+.1} dB", out_db),
            Style::default().fg(if out_db > -3.0 { HOT } else { CHROME }),
        ),
    ]));
    f.render_widget(scale_line, rows[2]);
}

fn render_vu_row(f: &mut Frame, area: Rect, label: &str, level: f32) {
    let db = amp_to_db(level);
    let fill = ((db + 60.0) / 60.0).clamp(0.0, 1.0) as f64;

    let bar_width = (area.width as usize).saturating_sub(label.len() + 2 + 10);
    let filled = (fill * bar_width as f64) as usize;

    let green_end = (bar_width as f64 * 0.72) as usize;
    let yellow_end = (bar_width as f64 * 0.88) as usize;

    let mut spans = vec![
        Span::styled(
            label,
            Style::default().fg(CHROME).add_modifier(Modifier::BOLD),
        ),
        Span::styled("▐", Style::default().fg(DIM)),
    ];

    for i in 0..bar_width {
        let ch = if i < filled { '█' } else { '░' };
        let color = if i < filled {
            if i < green_end {
                SAFE
            } else if i < yellow_end {
                WARN
            } else {
                HOT
            }
        } else {
            Color::Rgb(30, 30, 30)
        };
        spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
    }
    spans.push(Span::styled("▌", Style::default().fg(DIM)));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_amp_selector(f: &mut Frame, area: Rect, params: &Params, focused: bool) {
    let border_color = if focused {
        ORANGE
    } else {
        Color::Rgb(60, 40, 0)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(inner);

    // ── Amp model selector ────────────────────────────────────────────────────
    // When an external AU is the active amp the built-in model is bypassed, so the
    // whole selector is dimmed to signal it has no effect until `Z` returns to it.
    let amp_model = params.amp_model();
    let label_color = if focused { AMBER } else { DIM };
    let amp_ext_active = params.amp_external_active.load(Relaxed);
    let amp_label_fg = if amp_ext_active { OFF } else { label_color };

    let mut amp_spans = vec![Span::styled(
        "  AMP  ",
        Style::default()
            .fg(amp_label_fg)
            .add_modifier(Modifier::BOLD),
    )];
    for m in [AmpModel::Marshall, AmpModel::Mesa, AmpModel::Randall] {
        let selected = m == amp_model;
        let style = if amp_ext_active {
            Style::default().fg(OFF)
        } else if selected {
            Style::default()
                .fg(ORANGE)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(Color::Rgb(80, 60, 0))
        };
        let (bl, br) = if selected {
            ("◀ ", " ▶")
        } else {
            ("[ ", " ]")
        };
        let bc = if amp_ext_active {
            OFF
        } else if selected {
            AMBER
        } else {
            DIM
        };
        amp_spans.push(Span::styled(bl, Style::default().fg(bc)));
        amp_spans.push(Span::styled(m.short_name(), style));
        amp_spans.push(Span::styled(br, Style::default().fg(bc)));
        amp_spans.push(Span::raw("  "));
    }
    if focused {
        let hint = if amp_ext_active {
            "Z → built-in"
        } else {
            "↑/↓  A"
        };
        amp_spans.push(Span::styled(hint, Style::default().fg(DIM)));
    }
    f.render_widget(Paragraph::new(Line::from(amp_spans)), cols[0]);

    // ── Cabinet model selector ────────────────────────────────────────────────
    // The built-in cab model has no effect when an external IR is the active cab, or
    // when an external amp is supplying its own cab (amp+cab mode) — dim the selector
    // in either case. (In amp-only mode the built-in cab is back in the path.)
    let cab_model = params.cab_model();
    let ext_active = params.cab_external_active.load(Relaxed);
    let cab_inactive = ext_active || cab_bypassed_by_amp(params);
    let label_fg = if cab_inactive { OFF } else { label_color };
    let mut cab_spans = vec![Span::styled(
        "  CAB  ",
        Style::default().fg(label_fg).add_modifier(Modifier::BOLD),
    )];
    for m in [CabModel::Mesa, CabModel::Marshall, CabModel::Orange] {
        let selected = m == cab_model;
        let style = if cab_inactive {
            Style::default().fg(OFF)
        } else if selected {
            Style::default()
                .fg(ORANGE)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(Color::Rgb(80, 60, 0))
        };
        let (bl, br) = if selected {
            ("◀ ", " ▶")
        } else {
            ("[ ", " ]")
        };
        let bc = if cab_inactive {
            OFF
        } else if selected {
            AMBER
        } else {
            DIM
        };
        cab_spans.push(Span::styled(bl, Style::default().fg(bc)));
        cab_spans.push(Span::styled(m.short_name(), style));
        cab_spans.push(Span::styled(br, Style::default().fg(bc)));
        cab_spans.push(Span::raw("  "));
    }
    if focused {
        let hint = if cab_inactive {
            "C → built-in"
        } else {
            "C to toggle"
        };
        cab_spans.push(Span::styled(hint, Style::default().fg(DIM)));
    }
    f.render_widget(Paragraph::new(Line::from(cab_spans)), cols[1]);
}

// ── Amplifier head + cabinet/mic ──────────────────────────────────────────────
fn render_amp(
    f: &mut Frame,
    area: Rect,
    params: &Params,
    focus: Option<usize>,
    ext_cab: Option<&str>,
    ext_amp: Option<&str>,
) {
    let amp_active = focus.is_some_and(|i| (AMP_START..AMP_END).contains(&i));
    let mic_active = focus.is_some_and(|i| (MIC_START..MIC_END).contains(&i));
    let border_color = if amp_active || mic_active {
        ORANGE
    } else {
        WARM
    };

    // The amp panel reflects the active amp: a loaded AU's name (with the tone-stack
    // knobs inert) or the built-in amp model.
    let amp_name = match ext_amp {
        Some(name) => format!("AU: {name}"),
        None => params.amp_model().name().to_uppercase(),
    };
    // The cabinet/mic panel reflects the active cab. An external amp supplying its own
    // cab (amp+cab mode) bypasses the whole cab stage; otherwise a loaded IR or the
    // built-in cab model is shown.
    let cab_bypassed = cab_bypassed_by_amp(params);
    let cab_name = if cab_bypassed {
        "PLUGIN CAB".to_owned()
    } else {
        match ext_cab {
            Some(name) => format!("IR: {name}"),
            None => params.cab_model().short_name().to_owned(),
        }
    };

    let left_title = Line::from(vec![
        Span::styled("┤ ", Style::default().fg(border_color)),
        Span::styled(
            amp_name,
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ├", Style::default().fg(border_color)),
    ]);
    let right_title = Line::from(vec![
        Span::styled("┤ 🎙 ", Style::default().fg(border_color)),
        Span::styled(
            cab_name,
            Style::default().fg(CHROME).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ├", Style::default().fg(border_color)),
    ])
    .right_aligned();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(border_color))
        .title(left_title)
        .title(right_title)
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Control panel (knobs + mic) on top, speaker grille below.
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(1)])
        .split(inner);

    let panel = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(30)])
        .split(parts[0]);

    // Amp tone stack knobs.
    let count = AMP_END - AMP_START;
    let knob_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            (0..count)
                .map(|_| Constraint::Ratio(1, count as u32))
                .collect::<Vec<_>>(),
        )
        .split(panel[0]);
    // The tone-stack knobs drive the built-in amp; a loaded AU brings its own gain and
    // tone controls (edited in the AU modal), so they are dimmed while it is active —
    // exactly as the mic knobs are while an external IR is up.
    let amp_live = ext_amp.is_none();
    for (i, ki) in (AMP_START..AMP_END).enumerate() {
        let val = (KNOBS[ki].param)(params).load(Relaxed);
        render_compact_knob(
            f,
            knob_cols[i],
            KNOBS[ki].label,
            val,
            focus == Some(ki),
            amp_live,
            AMBER,
        );
    }

    // Cabinet mics (position, dynamic↔ribbon blend, room) in front of the cabinet.
    let mic_count = MIC_END - MIC_START;
    let mic_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            (0..mic_count)
                .map(|_| Constraint::Ratio(1, mic_count as u32))
                .collect::<Vec<_>>(),
        )
        .split(panel[1]);
    // The mic knobs only colour the built-in cab's multi-mic blend; they are inert when
    // a finished IR is the cab, or when an external amp supplies its own cab.
    let mic_live = ext_cab.is_none() && !cab_bypassed;
    for (i, ki) in (MIC_START..MIC_END).enumerate() {
        let val = (KNOBS[ki].param)(params).load(Relaxed);
        render_compact_knob(
            f,
            mic_cols[i],
            KNOBS[ki].label,
            val,
            focus == Some(ki),
            mic_live,
            CHROME,
        );
    }

    render_grille(f, parts[1], shade(WARM, 0.45));
}

fn render_grille(f: &mut Frame, area: Rect, color: Color) {
    let w = area.width as usize;
    let lines: Vec<Line> = (0..area.height as usize)
        .map(|row| {
            let s: String = (0..w)
                .map(|col| if (row + col) % 2 == 0 { '▚' } else { '▞' })
                .collect();
            Line::from(Span::styled(s, Style::default().fg(color)))
        })
        .collect();
    f.render_widget(Paragraph::new(lines), area);
}

// ── Guitar rig (pedalboard) ───────────────────────────────────────────────────
// Master–detail layout: a compact tile per pedal (name + LED + values) across
// the top, and a full-size dial editor for the focused pedal below. Screen cost
// is flat in pedal count — adding pedals grows the tile grid, not the editor.
fn render_rig(f: &mut Frame, area: Rect, params: &Params, board: &[bool], focus: Option<usize>) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(WARM))
        .title(Line::from(Span::styled(
            " GUITAR RIG ",
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Only on-board pedals get tiles; the last tile is always "+ ADD". `board`
    // may be shorter than PEDALS (e.g. the device-setup screen passes an empty
    // slice), so treat missing entries as off-board.
    let on_board: Vec<usize> = (0..PEDALS.len())
        .filter(|&i| board.get(i).copied().unwrap_or(false))
        .collect();
    let tile_count = on_board.len() + 1;

    // Tiles up top (fixed height), full-size editor below (takes the rest).
    const TILE_H: u16 = 4;
    let cols = ((inner.width / 16).max(1) as usize).min(tile_count);
    let tile_rows = tile_count.div_ceil(cols);
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(tile_rows as u16 * TILE_H),
            Constraint::Min(0),
        ])
        .split(inner);

    let grid_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(TILE_H); tile_rows])
        .split(parts[0]);

    for r in 0..tile_rows {
        let base = r * cols;
        let n = cols.min(tile_count - base);
        let cells = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Ratio(1, cols as u32); cols])
            .split(grid_rows[r]);
        for (c, cell) in cells.iter().take(n).enumerate() {
            match on_board.get(base + c) {
                Some(&pi) => render_pedal_tile(f, *cell, &PEDALS[pi], focus, params),
                None => render_add_tile(f, *cell, focus == Some(ADD_TILE)),
            }
        }
    }

    render_pedal_detail(f, parts[1], params, focus);
}

/// The "+ ADD" tile: an empty slot inviting the user to add a pedal.
fn render_add_tile(f: &mut Frame, area: Rect, focused: bool) {
    let color = if focused { AMBER } else { DIM };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(color))
        .title(Line::from(Span::styled(
            " + ADD ",
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "＋",
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center),
        parts[0],
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            if focused { "Enter" } else { "" },
            Style::default().fg(DIM),
        )))
        .alignment(Alignment::Center),
        parts[1],
    );
}

/// A compact pedal tile: name + LED in the title, all knob values on one line,
/// and a footswitch. The focused pedal's tile lights up to its full livery; the
/// rest dim by on/off state.
fn render_pedal_tile(
    f: &mut Frame,
    area: Rect,
    pedal: &Pedal,
    focus: Option<usize>,
    params: &Params,
) {
    let on = (pedal.enabled)(params).load(Relaxed);
    let active = focus.is_some_and(|i| (pedal.start..pedal.end).contains(&i));
    let body = if active {
        pedal.color
    } else if on {
        shade(pedal.color, 0.8)
    } else {
        shade(pedal.color, 0.35)
    };
    let name_color = if on {
        pedal.color
    } else {
        shade(pedal.color, 0.5)
    };

    let led = if on {
        Span::styled(
            "◉",
            Style::default()
                .fg(Color::Rgb(255, 70, 70))
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("○", Style::default().fg(OFF))
    };

    let title = Line::from(vec![
        Span::styled(
            format!(" {} ", pedal.name),
            Style::default().fg(name_color).add_modifier(Modifier::BOLD),
        ),
        led,
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(body))
        .title(title)
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let values: String = (pedal.start..pedal.end)
        .map(|ki| format!("{:.1}", (KNOBS[ki].param)(params).load(Relaxed) * 10.0))
        .collect::<Vec<_>>()
        .join("  ");
    let value_color = if active || on { pedal.color } else { OFF };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            values,
            Style::default()
                .fg(value_color)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center),
        parts[0],
    );

    let foot_color = if on { body } else { shade(pedal.color, 0.3) };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "▗▄▄▄▄▄▄▄▖",
            Style::default().fg(foot_color),
        )))
        .alignment(Alignment::Center),
        parts[1],
    );
}

/// The detail editor: full-size dials for whichever pedal currently has focus.
/// When focus is elsewhere (amp/mic/selectors) it shows a hint instead.
fn render_pedal_detail(f: &mut Frame, area: Rect, params: &Params, focus: Option<usize>) {
    let pedal = PEDALS
        .iter()
        .find(|p| focus.is_some_and(|i| (p.start..p.end).contains(&i)));

    // The editor takes on the focused pedal's livery; otherwise it stays dim.
    let border_color = pedal.map_or(DIM, |p| p.color);
    let adding = focus == Some(ADD_TILE);
    let title = match pedal {
        Some(p) => Line::from(vec![
            Span::styled("┤ EDITING: ", Style::default().fg(border_color)),
            Span::styled(
                p.name,
                Style::default().fg(p.color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ├", Style::default().fg(border_color)),
        ]),
        None if adding => Line::from(Span::styled(
            "┤ ADD A PEDAL — Enter ├",
            Style::default().fg(AMBER),
        )),
        None => Line::from(Span::styled(
            "┤ SELECT A PEDAL — Tab / ←→ ├",
            Style::default().fg(DIM),
        )),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(pedal) = pedal else {
        let hint = if adding {
            "Press Enter to add a pedal to the board."
        } else {
            "Tab to a pedal to edit its controls."
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(hint, Style::default().fg(DIM))))
                .alignment(Alignment::Center),
            inner,
        );
        return;
    };

    let on = (pedal.enabled)(params).load(Relaxed);
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(inner);

    let count = pedal.end - pedal.start;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Ratio(1, count as u32); count])
        .split(parts[0]);

    for (i, ki) in (pedal.start..pedal.end).enumerate() {
        let val = (KNOBS[ki].param)(params).load(Relaxed);
        render_compact_knob(
            f,
            cols[i],
            KNOBS[ki].label,
            val,
            focus == Some(ki),
            on,
            pedal.color,
        );
    }

    let foot_color = if on {
        pedal.color
    } else {
        shade(pedal.color, 0.3)
    };
    let foot = if on {
        "▐ ON ▌  ▗▄▄▄▄▄▄▄▖"
    } else {
        "○ OFF   ▗▄▄▄▄▄▄▄▖"
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            foot,
            Style::default().fg(foot_color),
        )))
        .alignment(Alignment::Center),
        parts[1],
    );
}

#[allow(clippy::too_many_arguments)]
fn render_compact_knob(
    f: &mut Frame,
    area: Rect,
    label: &str,
    value: f32,
    focused: bool,
    active: bool,
    accent: Color,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Length(1)])
        .split(area);

    let dial_color = if focused {
        AMBER
    } else if active {
        accent
    } else {
        OFF
    };

    let dial_h = (rows[0].height as usize).clamp(2, 5);
    let art: Vec<Line> = build_dial(value, focused, dial_h)
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(dial_color))))
        .collect();
    f.render_widget(Paragraph::new(art).alignment(Alignment::Center), rows[0]);

    let num = value * 10.0;
    let label_color = if focused {
        AMBER
    } else if active {
        DIM
    } else {
        OFF
    };
    let value_color = if focused {
        ORANGE
    } else if active {
        accent
    } else {
        OFF
    };

    let label_line = Line::from(vec![
        Span::styled(
            format!("{label} "),
            Style::default()
                .fg(label_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{num:.1}"),
            Style::default()
                .fg(value_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(
        Paragraph::new(label_line).alignment(Alignment::Center),
        rows[1],
    );
}

fn render_help(f: &mut Frame, area: Rect, status: Option<&str>) {
    let line = if let Some(msg) = status {
        Line::from(vec![Span::styled(
            format!(" {msg} "),
            Style::default().fg(SAFE).add_modifier(Modifier::BOLD),
        )])
    } else {
        let mut spans = vec![
            Span::styled(" Tab ", Style::default().fg(AMBER)),
            Span::styled("section  ", Style::default().fg(DIM)),
            Span::styled("←/→", Style::default().fg(AMBER)),
            Span::styled(" knob  ", Style::default().fg(DIM)),
            Span::styled("↑/↓  +/-", Style::default().fg(AMBER)),
            Span::styled(" adjust  ", Style::default().fg(DIM)),
            Span::styled("Space", Style::default().fg(AMBER)),
            Span::styled(" toggle  ", Style::default().fg(DIM)),
            Span::styled("D", Style::default().fg(AMBER)),
            Span::styled(" remove  ", Style::default().fg(DIM)),
            Span::styled("A", Style::default().fg(AMBER)),
            Span::styled(" amp  ", Style::default().fg(DIM)),
            Span::styled("C", Style::default().fg(AMBER)),
            Span::styled(" cab  ", Style::default().fg(DIM)),
            Span::styled("I", Style::default().fg(AMBER)),
            Span::styled(" IR  ", Style::default().fg(DIM)),
            Span::styled("P", Style::default().fg(AMBER)),
            Span::styled(" presets  ", Style::default().fg(DIM)),
            Span::styled("T", Style::default().fg(AMBER)),
            Span::styled(" tuner  ", Style::default().fg(DIM)),
        ];
        #[cfg(feature = "clap")]
        {
            spans.push(Span::styled("V", Style::default().fg(AMBER)));
            spans.push(Span::styled(" plugins  ", Style::default().fg(DIM)));
        }
        // Keyed on the feature (not the OS) so the footer renders identically on every
        // platform — matching the `V plugins` hint above and keeping the golden
        // snapshots portable. The key itself only does anything on macOS, where AU
        // hosting is compiled in.
        #[cfg(feature = "au")]
        {
            spans.push(Span::styled("U", Style::default().fg(AMBER)));
            spans.push(Span::styled(" amp plugin  ", Style::default().fg(DIM)));
        }
        spans.extend([
            Span::styled("R", Style::default().fg(AMBER)),
            Span::styled(" record  ", Style::default().fg(DIM)),
            Span::styled("Q", Style::default().fg(AMBER)),
            Span::styled(" quit", Style::default().fg(DIM)),
        ]);
        Line::from(spans)
    };
    let help = Paragraph::new(line)
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black));
    f.render_widget(help, area);
}

/// Modal listing the pedals not currently on the board. `available` holds their
/// `PEDALS` indices; `cursor` is the highlighted row.
pub(super) fn render_add_pedal_modal(f: &mut Frame, available: &[usize], cursor: usize) {
    let area = {
        let a = f.area();
        let width = (a.width * 45 / 100).max(24);
        let height = ((available.len() as u16 + 4).min(a.height)).max(6);
        Rect {
            x: a.x + (a.width - width) / 2,
            y: a.y + (a.height - height) / 2,
            width,
            height,
        }
    };
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(
            " A D D   P E D A L ",
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    if available.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "All pedals are on the board.",
                Style::default().fg(DIM),
            )))
            .alignment(Alignment::Center),
            rows[0],
        );
    } else {
        let lines: Vec<Line> = available
            .iter()
            .enumerate()
            .map(|(i, &pi)| {
                let p = &PEDALS[pi];
                let selected = i == cursor;
                let (prefix, style) = if selected {
                    (
                        "▶ ",
                        Style::default()
                            .fg(p.color)
                            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
                    )
                } else {
                    ("  ", Style::default().fg(p.color))
                };
                Line::from(vec![
                    Span::styled(
                        prefix,
                        Style::default().fg(if selected { ORANGE } else { DIM }),
                    ),
                    Span::styled(p.name, style),
                ])
            })
            .collect();
        f.render_widget(Paragraph::new(lines), rows[0]);
    }

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(AMBER)),
            Span::styled(" navigate  ", Style::default().fg(DIM)),
            Span::styled("Enter", Style::default().fg(AMBER)),
            Span::styled(" add  ", Style::default().fg(DIM)),
            Span::styled("Esc", Style::default().fg(AMBER)),
            Span::styled(" close", Style::default().fg(DIM)),
        ]))
        .alignment(Alignment::Center),
        rows[1],
    );
}

/// Builds an ASCII rotary knob `rows` lines tall: a hub, a dotted rim, and a
/// pointer "hand" that swings 270° (7 o'clock → 5 o'clock) as `value` goes
/// 0.0 → 1.0. The hand is a real line whose glyph and direction track the
/// value, so even neighbouring settings look visibly different.
fn build_dial(value: f32, focused: bool, rows: usize) -> Vec<String> {
    use std::f32::consts::PI;

    let rows = rows.max(2);
    let cols = rows * 2 - 1;

    let start_deg = 225.0_f32;
    let sweep = 270.0_f32;
    let deg = start_deg - value.clamp(0.0, 1.0) * sweep;
    let angle = deg * PI / 180.0;

    // rx:ry = 2:1 compensates for terminal char aspect ratio (~2x taller than wide)
    let cx = (cols as f32 - 1.0) / 2.0;
    let cy = (rows as f32 - 1.0) / 2.0;
    let rx = cx.max(1.0);
    let ry = cy.max(0.5);

    let mut grid = vec![vec![' '; cols]; rows];
    let put = |grid: &mut Vec<Vec<char>>, x: isize, y: isize, ch: char| {
        if x >= 0 && (x as usize) < cols && y >= 0 && (y as usize) < rows {
            grid[y as usize][x as usize] = ch;
        }
    };

    // Dotted rim, drawn only across the live 270° sweep.
    if rows >= 3 {
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                let dx = (col as f32 - cx) / rx;
                let dy = (row as f32 - cy) / ry;
                let dist = (dx * dx + dy * dy).sqrt();
                if (dist - 1.0).abs() < 0.35 {
                    let a = (-(row as f32 - cy)).atan2(col as f32 - cx).to_degrees();
                    let rel = (start_deg - a).rem_euclid(360.0);
                    if rel <= sweep {
                        put(&mut grid, col, row, '·');
                    }
                }
            }
        }
    }

    // Pointer. Small (pedal) dials draw a full "hand" line from the hub to the
    // rim so their limited resolution still reads as rotation; larger (amp)
    // dials stay clean with just a tip marker at the rim.
    let tip = if focused { '◆' } else { '◇' };

    let x = (cx + angle.cos() * rx).round() as isize;
    let y = (cy - angle.sin() * ry).round() as isize;
    put(&mut grid, x, y, tip);

    // Center hub last so it always shows.
    put(&mut grid, cx.round() as isize, cy.round() as isize, '●');

    grid.into_iter().map(|r| r.into_iter().collect()).collect()
}

/// Scales an RGB color toward black by `factor` (0.0 = black, 1.0 = unchanged).
fn shade(c: Color, factor: f32) -> Color {
    match c {
        Color::Rgb(r, g, b) => Color::Rgb(
            (f32::from(r) * factor) as u8,
            (f32::from(g) * factor) as u8,
            (f32::from(b) * factor) as u8,
        ),
        other => other,
    }
}

/// Whether an external amp is active *and* supplying its own cab (amp+cab mode), so the
/// built-in cabinet stage — model selector and mic knobs — is bypassed. False in
/// amp-only mode, where the built-in cab/IR stays in the path.
fn cab_bypassed_by_amp(params: &Params) -> bool {
    params.amp_external_active.load(Relaxed) && !params.amp_external_amp_only.load(Relaxed)
}

fn amp_to_db(amp: f32) -> f32 {
    if amp < 1e-6 {
        -120.0
    } else {
        20.0 * amp.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::{AmpModel, CabModel, Levels};
    use ratatui::{Terminal, backend::TestBackend};

    /// A comfortably large canvas so nothing the tests assert on is clipped by a
    /// small terminal. The layout is responsive, so exact dimensions only matter
    /// for the golden snapshot (which is re-blessed with `cargo insta review`).
    const W: u16 = 170;
    const H: u16 = 55;

    /// Flatten the rendered cells into plain text, one row per line. Styles are
    /// dropped — layout invariants live in the config tests, this checks the
    /// visible glyphs.
    fn screen_text(term: &Terminal<TestBackend>) -> String {
        let buf = term.backend().buffer();
        let area = buf.area();
        let mut out = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    /// Render the main screen with the given board/focus and return its glyphs.
    fn render(board: &[bool], focus: Option<usize>) -> (Terminal<TestBackend>, String) {
        let params = Params::new();
        let text = render_with(&params, board, focus, |_| {});
        (
            Terminal::new(TestBackend::new(W, H)).expect("test backend"),
            text,
        )
    }

    /// Render the main screen for `params`, then let `overlay` draw a modal on top
    /// (exactly as the event loop composites modals over the board), and return the
    /// flattened glyphs.
    fn render_with(
        params: &Params,
        board: &[bool],
        focus: Option<usize>,
        overlay: impl FnOnce(&mut Frame),
    ) -> String {
        let levels = Levels::new();
        let mut term = Terminal::new(TestBackend::new(W, H)).expect("test backend");
        term.draw(|f| {
            draw(
                f, params, &levels, focus, board, false, false, None, None, None, None,
            );
            overlay(f);
        })
        .expect("draw");
        screen_text(&term)
    }

    fn board_all(on: bool) -> Vec<bool> {
        vec![on; PEDALS.len()]
    }

    /// The board the app actually boots with, derived from the default enabled
    /// flags (TS-808 + Noise Gate on) exactly like `sync_board` does at startup.
    /// Only the `clap`-gated golden tests use it.
    #[cfg(feature = "clap")]
    fn default_board(params: &Params) -> Vec<bool> {
        use std::sync::atomic::Ordering::Relaxed;
        PEDALS
            .iter()
            .map(|p| (p.enabled)(params).load(Relaxed))
            .collect()
    }

    /// Golden snapshot of a realistic default screen — the board the app boots
    /// with (TS-808 + Noise Gate tiles), focused on the first on-board pedal so
    /// the detail editor's dials are captured too. This is the tripwire for the
    /// overall layout: any unintended change to spacing, labels, tiles, or dials
    /// shows up as a diff; intentional changes are re-blessed with
    /// `cargo insta accept`.
    ///
    /// Gated on `clap`: the help footer's `V plugins` key is `clap`-only, so the
    /// rendered chrome (and thus every golden below) is specific to the default
    /// build the snapshots were captured in. CI runs default features.
    #[cfg(feature = "clap")]
    #[test]
    fn snapshot_default_screen() {
        let params = Params::new();
        let board = default_board(&params);
        let first_on = board.iter().position(|&on| on).expect("a default pedal");
        let text = render_with(&params, &board, Some(PEDALS[first_on].start), |_| {});
        insta::assert_snapshot!("default_screen", text);
    }

    /// Rendering must never panic across a spread of states: empty board, full
    /// board, focus on a pedal knob, focus on the +ADD tile, and recording on.
    #[test]
    fn rendering_is_panic_free_across_states() {
        let params = Params::new();
        let levels = Levels::new();
        let cases: [(Vec<bool>, Option<usize>, bool); 4] = [
            (board_all(false), None, false),
            (board_all(true), Some(PEDALS[0].start), false),
            (board_all(true), Some(ADD_TILE), true),
            (board_all(false), Some(AMP_START), true),
        ];
        for (board, focus, rec) in cases {
            let mut term = Terminal::new(TestBackend::new(W, H)).expect("test backend");
            term.draw(|f| {
                draw(
                    f,
                    &params,
                    &levels,
                    focus,
                    &board,
                    rec,
                    true,
                    Some("REC…"),
                    None,
                    None,
                    None,
                );
            })
            .expect("draw");
        }
    }

    /// Every pedal must render intact: focusing each one shows the detail editor
    /// titled with that pedal's full name and every one of its knob labels.
    #[test]
    fn every_pedal_renders_with_its_name_and_knob_labels() {
        for (pi, pedal) in PEDALS.iter().enumerate() {
            let mut board = board_all(false);
            board[pi] = true;
            let (_term, text) = render(&board, Some(pedal.start));
            assert!(
                text.contains(pedal.name),
                "{} name missing from the detail editor",
                pedal.name
            );
            for knob in KNOBS.iter().take(pedal.end).skip(pedal.start) {
                assert!(
                    text.contains(knob.label),
                    "{}: knob label {:?} missing",
                    pedal.name,
                    knob.label
                );
            }
        }
    }

    /// Every amp model must be reachable and shown in the header/selector.
    #[test]
    fn every_amp_model_renders_its_name() {
        for model in [AmpModel::Marshall, AmpModel::Mesa, AmpModel::Randall] {
            let params = Params::new();
            params
                .amp_model
                .store(model as u8, std::sync::atomic::Ordering::Relaxed);
            let levels = Levels::new();
            let mut term = Terminal::new(TestBackend::new(W, H)).expect("test backend");
            term.draw(|f| {
                draw(
                    f,
                    &params,
                    &levels,
                    None,
                    &board_all(false),
                    false,
                    false,
                    None,
                    None,
                    None,
                    None,
                );
            })
            .expect("draw");
            let text = screen_text(&term);
            assert!(
                text.contains(model.short_name()),
                "amp {:?} not shown on screen",
                model.short_name()
            );
        }
    }

    /// An active external amp (hosted AU) must be surfaced in the header in place of
    /// the built-in amp model — the visual counterpart to the external-IR "IR:" label.
    #[test]
    fn external_amp_name_shows_in_header() {
        let params = Params::new();
        params
            .amp_external_active
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let levels = Levels::new();
        let mut term = Terminal::new(TestBackend::new(W, H)).expect("test backend");
        term.draw(|f| {
            draw(
                f,
                &params,
                &levels,
                None,
                &board_all(false),
                false,
                false,
                None,
                None,
                None,
                Some("Silver Jubilee"),
            );
        })
        .expect("draw");
        let text = screen_text(&term);
        assert!(
            text.contains("AU: SILVER JUBILEE"),
            "external amp name not shown in the header"
        );
        // The built-in amp model label must not also be shown as the active amp.
        assert!(
            !text.contains("MARSHALL JCM800"),
            "built-in amp label shown while an external amp is active"
        );
    }

    /// Every built-in cabinet must be reachable and shown in the selector.
    #[test]
    fn every_cab_model_renders_its_name() {
        for model in [CabModel::Mesa, CabModel::Marshall, CabModel::Orange] {
            let params = Params::new();
            params
                .cab_model
                .store(model as u8, std::sync::atomic::Ordering::Relaxed);
            let levels = Levels::new();
            let mut term = Terminal::new(TestBackend::new(W, H)).expect("test backend");
            term.draw(|f| {
                draw(
                    f,
                    &params,
                    &levels,
                    None,
                    &board_all(false),
                    false,
                    false,
                    None,
                    None,
                    None,
                    None,
                );
            })
            .expect("draw");
            let text = screen_text(&term);
            assert!(
                text.contains(model.short_name()),
                "cab {:?} not shown on screen",
                model.short_name()
            );
        }
    }

    /// The +ADD pedal modal lists every off-board pedal by name.
    #[test]
    fn add_pedal_modal_lists_available_pedals() {
        let params = Params::new();
        let levels = Levels::new();
        let board = board_all(false);
        let available: Vec<usize> = (0..PEDALS.len()).collect();
        let mut term = Terminal::new(TestBackend::new(W, H)).expect("test backend");
        term.draw(|f| {
            draw(
                f, &params, &levels, None, &board, false, false, None, None, None, None,
            );
            render_add_pedal_modal(f, &available, 0);
        })
        .expect("draw");
        let text = screen_text(&term);
        for &pi in &available {
            assert!(
                text.contains(PEDALS[pi].name),
                "{} missing from the add-pedal modal",
                PEDALS[pi].name
            );
        }
    }

    // ── modal golden snapshots ──────────────────────────────────────────────────
    // Each modal is composited over the realistic default board, just like the
    // event loop draws it. Inputs are fixed in-test (no filesystem), so the goldens
    // are deterministic across machines and CI.

    /// The +ADD picker, over the default board (so its list is the off-board
    /// pedals), cursor on the first entry.
    #[cfg(feature = "clap")]
    #[test]
    fn snapshot_add_pedal_modal() {
        let params = Params::new();
        let board = default_board(&params);
        let available: Vec<usize> = (0..PEDALS.len()).filter(|&i| !board[i]).collect();
        let text = render_with(&params, &board, None, |f| {
            render_add_pedal_modal(f, &available, 0);
        });
        insta::assert_snapshot!("add_pedal_modal", text);
    }

    /// The preset picker with a fixed System + User preset, cursor on the user
    /// entry (which reveals the `D delete` footer hint and the `[user]` tag).
    #[cfg(feature = "clap")]
    #[test]
    fn snapshot_preset_modal() {
        use crate::preset::{Preset, PresetSource};
        let params = Params::new();
        let mut system = Preset::from_params(
            "Clean Combo".to_string(),
            Some("sparkly cleans".to_string()),
            &params,
        );
        system.source = PresetSource::System;
        let mut user = Preset::from_params(
            "My Lead".to_string(),
            Some("saved rig".to_string()),
            &params,
        );
        user.source = PresetSource::User;
        let presets = vec![system, user];
        let board = default_board(&params);
        // Entries: [Default values, Clean Combo, My Lead] → cursor 2 = the user one.
        let text = render_with(&params, &board, None, |f| {
            crate::ui::presets::render_preset_modal(f, &presets, 2);
        });
        insta::assert_snapshot!("preset_modal", text);
    }

    /// The save-preset dialog with both fields filled, focus on the name field.
    #[cfg(feature = "clap")]
    #[test]
    fn snapshot_save_preset_dialog() {
        let params = Params::new();
        let board = default_board(&params);
        let text = render_with(&params, &board, None, |f| {
            crate::ui::presets::render_save_dialog(f, "My Lead", "warm mid-gain", 0, None);
        });
        insta::assert_snapshot!("save_preset_dialog", text);
    }

    /// The CLAP plugin browser (Browse view). The list is left empty on purpose —
    /// `open = true` shows the modal WITHOUT calling `open()`, which would scan the
    /// filesystem and make the golden machine-dependent.
    #[cfg(feature = "clap")]
    #[test]
    fn snapshot_plugin_browser_modal() {
        let params = Params::new();
        let mut browser = crate::ui::plugins::PluginBrowser::new(48_000.0, 512);
        browser.open = true;
        let board = default_board(&params);
        let text = render_with(&params, &board, None, |f| browser.render(f));
        insta::assert_snapshot!("plugin_browser_modal", text);
    }

    /// The external-IR browser. As with the plugin browser, the file list is left
    /// empty (no scan) so the golden captures the deterministic empty state.
    #[cfg(feature = "clap")]
    #[test]
    fn snapshot_ir_browser_modal() {
        let params = Params::new();
        let mut browser = crate::ui::ir_browser::IrBrowser::new(48_000.0);
        browser.open = true;
        let board = default_board(&params);
        let text = render_with(&params, &board, None, |f| browser.render(f, false));
        insta::assert_snapshot!("ir_browser_modal", text);
    }

    /// With an external amp active, the IR browser must warn that IRs have no effect
    /// (the AU replaces the built-in amp+cab) rather than silently doing nothing.
    #[test]
    fn ir_browser_warns_when_external_amp_active() {
        let params = Params::new();
        let mut browser = crate::ui::ir_browser::IrBrowser::new(48_000.0);
        browser.open = true;
        let text = render_with(&params, &board_all(false), None, |f| {
            browser.render(f, true)
        });
        assert!(
            text.contains("External amp active"),
            "IR browser should warn while an external amp is active"
        );
    }
}
