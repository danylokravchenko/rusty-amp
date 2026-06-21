use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::{AmpModel, CabModel, Levels, Params};

use super::config::{
    AMP_END, AMP_START, DELAY_END, DELAY_START, DS_END, DS_START, EQ_END, EQ_START, FUZZ_END,
    FUZZ_START, KNOBS, MIC_END, MIC_START, NG_END, NG_START, REV_END, REV_START, TS_END, TS_START,
};
use super::styles::*;

pub(super) fn draw(
    f: &mut Frame,
    params: &Params,
    levels: &Levels,
    focus: Option<usize>,
    recording: bool,
    blink: bool,
    status: Option<&str>,
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

    render_header(f, rows[0], params, recording, blink);
    render_meters(f, rows[1], levels);
    render_amp_selector(f, rows[2], params, focus.is_none());
    render_amp(f, rows[3], params, focus);
    render_rig(f, rows[4], params, focus);
    render_help(f, rows[5], status);
}

fn render_header(f: &mut Frame, area: Rect, params: &Params, recording: bool, blink: bool) {
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

    let amp_name = params.amp_model().name().to_uppercase();
    let chain = format!("  TS-808 → {} → REVERB  ", params.amp_model().name());

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "  R U S T Y  A M P  ",
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ),
        Span::styled("▐", Style::default().fg(WARM)),
        Span::styled(
            format!("  {amp_name}  "),
            Style::default().fg(CHROME).add_modifier(Modifier::BOLD),
        ),
        Span::styled("▐", Style::default().fg(WARM)),
        Span::styled(chain, Style::default().fg(DIM)),
        Span::styled("▐", Style::default().fg(WARM)),
        Span::styled(
            "  ● POWER ON  ",
            Style::default().fg(SAFE).add_modifier(Modifier::BOLD),
        ),
        Span::styled("▐", Style::default().fg(WARM)),
        if recording && blink {
            Span::styled(
                "  ● ON AIR  ",
                Style::default().fg(HOT).add_modifier(Modifier::BOLD),
            )
        } else if recording {
            Span::styled("  ○ ON AIR  ", Style::default().fg(HOT))
        } else {
            Span::styled("  ○ OFF AIR  ", Style::default().fg(OFF))
        },
    ]));
    f.render_widget(title, rows[0]);

    let ng_on = params.ng_enabled.load(Relaxed);
    let fz_on = params.fz_enabled.load(Relaxed);
    let ts_on = params.ts_enabled.load(Relaxed);
    let ds_on = params.ds_enabled.load(Relaxed);
    let eq_on = params.eq_enabled.load(Relaxed);
    let delay_on = params.delay_enabled.load(Relaxed);
    let rev_on = params.rev_enabled.load(Relaxed);

    let arrow = Span::styled(" ──▶ ", Style::default().fg(DIM));

    let pedal_color = |on: bool| if on { ORANGE } else { OFF };
    let amp_color = AMBER;

    let mut chain: Vec<Span> = vec![Span::raw("  ")];

    chain.push(Span::styled(
        "GATE",
        Style::default().fg(pedal_color(ng_on)),
    ));
    chain.push(arrow.clone());
    chain.push(Span::styled(
        "FUZZ",
        Style::default().fg(pedal_color(fz_on)),
    ));
    chain.push(arrow.clone());
    chain.push(Span::styled(
        "TS-808",
        Style::default().fg(pedal_color(ts_on)),
    ));
    chain.push(arrow.clone());
    chain.push(Span::styled(
        "DS-1",
        Style::default().fg(pedal_color(ds_on)),
    ));
    chain.push(arrow.clone());
    chain.push(Span::styled("AMP", Style::default().fg(amp_color)));
    chain.push(arrow.clone());
    chain.push(Span::styled("CAB", Style::default().fg(amp_color)));
    chain.push(arrow.clone());
    chain.push(Span::styled("EQ", Style::default().fg(pedal_color(eq_on))));
    chain.push(arrow.clone());
    chain.push(Span::styled(
        "DELAY",
        Style::default().fg(pedal_color(delay_on)),
    ));
    chain.push(arrow.clone());
    chain.push(Span::styled(
        "REVERB",
        Style::default().fg(pedal_color(rev_on)),
    ));
    chain.push(arrow.clone());
    chain.push(Span::styled("OUTPUT", Style::default().fg(CHROME)));

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
    let amp_model = params.amp_model();
    let label_color = if focused { AMBER } else { DIM };

    let mut amp_spans = vec![Span::styled(
        "  AMP  ",
        Style::default()
            .fg(label_color)
            .add_modifier(Modifier::BOLD),
    )];
    for m in [AmpModel::Marshall, AmpModel::Mesa, AmpModel::Randall] {
        let selected = m == amp_model;
        let style = if selected {
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
        let bc = if selected { AMBER } else { DIM };
        amp_spans.push(Span::styled(bl, Style::default().fg(bc)));
        amp_spans.push(Span::styled(m.short_name(), style));
        amp_spans.push(Span::styled(br, Style::default().fg(bc)));
        amp_spans.push(Span::raw("  "));
    }
    if focused {
        amp_spans.push(Span::styled("↑/↓  A", Style::default().fg(DIM)));
    }
    f.render_widget(Paragraph::new(Line::from(amp_spans)), cols[0]);

    // ── Cabinet model selector ────────────────────────────────────────────────
    let cab_model = params.cab_model();
    let mut cab_spans = vec![Span::styled(
        "  CAB  ",
        Style::default()
            .fg(label_color)
            .add_modifier(Modifier::BOLD),
    )];
    for m in [CabModel::Mesa, CabModel::Marshall] {
        let selected = m == cab_model;
        let style = if selected {
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
        let bc = if selected { AMBER } else { DIM };
        cab_spans.push(Span::styled(bl, Style::default().fg(bc)));
        cab_spans.push(Span::styled(m.short_name(), style));
        cab_spans.push(Span::styled(br, Style::default().fg(bc)));
        cab_spans.push(Span::raw("  "));
    }
    if focused {
        cab_spans.push(Span::styled("C to toggle", Style::default().fg(DIM)));
    }
    f.render_widget(Paragraph::new(Line::from(cab_spans)), cols[1]);
}

// ── Amplifier head + cabinet/mic ──────────────────────────────────────────────
fn render_amp(f: &mut Frame, area: Rect, params: &Params, focus: Option<usize>) {
    let amp_active = focus.is_some_and(|i| (AMP_START..AMP_END).contains(&i));
    let mic_active = focus.is_some_and(|i| (MIC_START..MIC_END).contains(&i));
    let border_color = if amp_active || mic_active {
        ORANGE
    } else {
        WARM
    };

    let amp_name = params.amp_model().name().to_uppercase();
    let cab_name = params.cab_model().short_name();

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
    for (i, ki) in (AMP_START..AMP_END).enumerate() {
        let val = (KNOBS[ki].param)(params).load(Relaxed);
        render_compact_knob(
            f,
            knob_cols[i],
            KNOBS[ki].label,
            val,
            focus == Some(ki),
            true,
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
    for (i, ki) in (MIC_START..MIC_END).enumerate() {
        let val = (KNOBS[ki].param)(params).load(Relaxed);
        render_compact_knob(
            f,
            mic_cols[i],
            KNOBS[ki].label,
            val,
            focus == Some(ki),
            true,
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
fn render_rig(f: &mut Frame, area: Rect, params: &Params, focus: Option<usize>) {
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

    // Split into two equal-height pedal rows so the second row (noise gate /
    // parametric EQ) gets dials the same size as the first. Any odd leftover
    // row is absorbed as a thin gap at the bottom rather than inflating row 1.
    let half = inner.height / 2;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(half),
            Constraint::Length(half),
            Constraint::Min(0),
        ])
        .split(inner);

    // Row 1: TS-808, DS-1, Reverb, Delay.
    let row1 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(rows[0]);

    render_pedal(
        f,
        row1[0],
        "TS-808",
        PEDAL_GREEN,
        TS_START,
        TS_END,
        params.ts_enabled.load(Relaxed),
        focus,
        params,
    );
    render_pedal(
        f,
        row1[1],
        "DS-1",
        PEDAL_ORANGE,
        DS_START,
        DS_END,
        params.ds_enabled.load(Relaxed),
        focus,
        params,
    );
    render_pedal(
        f,
        row1[2],
        "SPRING REVERB",
        PEDAL_BLUE,
        REV_START,
        REV_END,
        params.rev_enabled.load(Relaxed),
        focus,
        params,
    );
    render_pedal(
        f,
        row1[3],
        "DELAY",
        PEDAL_PURPLE,
        DELAY_START,
        DELAY_END,
        params.delay_enabled.load(Relaxed),
        focus,
        params,
    );

    // Row 2: Fuzz, Noise Gate, Parametric EQ.
    let row2 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(4, 12),
            Constraint::Ratio(3, 12),
            Constraint::Ratio(5, 12),
        ])
        .split(rows[1]);

    render_pedal(
        f,
        row2[0],
        "FUZZ",
        PEDAL_RED,
        FUZZ_START,
        FUZZ_END,
        params.fz_enabled.load(Relaxed),
        focus,
        params,
    );
    render_pedal(
        f,
        row2[1],
        "NOISE GATE",
        PEDAL_SILVER,
        NG_START,
        NG_END,
        params.ng_enabled.load(Relaxed),
        focus,
        params,
    );
    render_pedal(
        f,
        row2[2],
        "PARAMETRIC EQ",
        PEDAL_TEAL,
        EQ_START,
        EQ_END,
        params.eq_enabled.load(Relaxed),
        focus,
        params,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_pedal(
    f: &mut Frame,
    area: Rect,
    name: &str,
    color: Color,
    start: usize,
    end: usize,
    on: bool,
    focus: Option<usize>,
    params: &Params,
) {
    let active = focus.is_some_and(|i| (start..end).contains(&i));
    let body = if active {
        color
    } else if on {
        shade(color, 0.8)
    } else {
        shade(color, 0.35)
    };
    let name_color = if on { color } else { shade(color, 0.5) };

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
            format!(" {name} "),
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
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(inner);

    let count = end - start;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            (0..count)
                .map(|_| Constraint::Ratio(1, count as u32))
                .collect::<Vec<_>>(),
        )
        .split(parts[0]);

    for (i, ki) in (start..end).enumerate() {
        let val = (KNOBS[ki].param)(params).load(Relaxed);
        render_compact_knob(
            f,
            cols[i],
            KNOBS[ki].label,
            val,
            focus == Some(ki),
            on,
            color,
        );
    }

    // Footswitch (stomp pad).
    let foot_color = if on { body } else { shade(color, 0.3) };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "▗▄▄▄▄▄▄▄▖",
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
        Line::from(vec![
            Span::styled(" Tab ", Style::default().fg(AMBER)),
            Span::styled("section  ", Style::default().fg(DIM)),
            Span::styled("←/→", Style::default().fg(AMBER)),
            Span::styled(" knob  ", Style::default().fg(DIM)),
            Span::styled("↑/↓  +/-", Style::default().fg(AMBER)),
            Span::styled(" adjust  ", Style::default().fg(DIM)),
            Span::styled("Space", Style::default().fg(AMBER)),
            Span::styled(" toggle  ", Style::default().fg(DIM)),
            Span::styled("A", Style::default().fg(AMBER)),
            Span::styled(" amp  ", Style::default().fg(DIM)),
            Span::styled("C", Style::default().fg(AMBER)),
            Span::styled(" cab  ", Style::default().fg(DIM)),
            Span::styled("P", Style::default().fg(AMBER)),
            Span::styled(" presets  ", Style::default().fg(DIM)),
            Span::styled("R", Style::default().fg(AMBER)),
            Span::styled(" record  ", Style::default().fg(DIM)),
            Span::styled("Q", Style::default().fg(AMBER)),
            Span::styled(" quit", Style::default().fg(DIM)),
        ])
    };
    let help = Paragraph::new(line)
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black));
    f.render_widget(help, area);
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

fn amp_to_db(amp: f32) -> f32 {
    if amp < 1e-6 {
        -120.0
    } else {
        20.0 * amp.log10()
    }
}
