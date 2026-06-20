use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::{AmpModel, CabModel, Levels, Params};

use super::config::{AMP_SECTIONS, KNOBS, PEDAL_SECTIONS};
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
        .style(Style::default().bg(ratatui::style::Color::Black));
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Ratio(1, 2),
            Constraint::Ratio(1, 2),
            Constraint::Length(1),
        ])
        .split(inner);

    render_header(f, rows[0], params, recording, blink);
    render_meters(f, rows[1], levels);
    render_amp_selector(f, rows[2], params, focus.is_none());
    render_section_row(f, rows[3], PEDAL_SECTIONS, params, focus);
    render_section_row(f, rows[4], AMP_SECTIONS, params, focus);
    render_help(f, rows[5], status);
}

fn render_header(f: &mut Frame, area: Rect, params: &Params, recording: bool, blink: bool) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(WARM))
        .style(Style::default().bg(ratatui::style::Color::Black));
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
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(WARM))
        .style(Style::default().bg(ratatui::style::Color::Black));
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
            ratatui::style::Color::Rgb(30, 30, 30)
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
        ratatui::style::Color::Rgb(60, 40, 0)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(ratatui::style::Color::Black));
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
            Style::default().fg(ratatui::style::Color::Rgb(80, 60, 0))
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
            Style::default().fg(ratatui::style::Color::Rgb(80, 60, 0))
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

fn render_section_row(
    f: &mut Frame,
    area: Rect,
    sections: &[super::config::SectionDef],
    params: &Params,
    focus: Option<usize>,
) {
    let total_weight: u32 = sections.iter().map(|s| s.4).sum();
    let constraints: Vec<Constraint> = sections
        .iter()
        .map(|s| Constraint::Ratio(s.4, total_weight))
        .collect();

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (i, (title_fn, start, end, enabled_fn, _)) in sections.iter().enumerate() {
        let title = title_fn(params);
        let enabled = enabled_fn(params);
        render_section(f, cols[i], &title, params, *start, *end, enabled, focus);
    }
}

#[allow(clippy::too_many_arguments)]
fn render_section(
    f: &mut Frame,
    area: Rect,
    title: &str,
    params: &Params,
    start: usize,
    end: usize,
    enabled: Option<bool>,
    focus: Option<usize>,
) {
    let focused_knob = focus.unwrap_or(usize::MAX);
    let active = focus.is_some_and(|f| f >= start && f < end);
    let is_on = enabled.unwrap_or(true);

    let border_color = if active {
        ORANGE
    } else if !is_on {
        ratatui::style::Color::Rgb(40, 30, 0)
    } else {
        ratatui::style::Color::Rgb(60, 40, 0)
    };

    let title_color = if !is_on {
        OFF
    } else if active {
        AMBER
    } else {
        DIM
    };
    let badge_color = if !is_on {
        OFF
    } else if active {
        SAFE
    } else {
        ratatui::style::Color::Rgb(0, 100, 0)
    };

    let title_spans = if let Some(on) = enabled {
        vec![
            Span::styled(
                format!(" {title} "),
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if on { "● " } else { "○ " },
                Style::default()
                    .fg(badge_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]
    } else {
        vec![Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )]
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(title_spans))
        .style(Style::default().bg(ratatui::style::Color::Black));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let count = end - start;
    let constraints: Vec<Constraint> = (0..count)
        .map(|_| Constraint::Ratio(1, count as u32))
        .collect();

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner);

    for (i, ki) in (start..end).enumerate() {
        let val = (KNOBS[ki].param)(params).load(Relaxed);
        render_knob(f, cols[i], KNOBS[ki].label, val, ki == focused_knob, is_on);
    }
}

fn render_knob(f: &mut Frame, area: Rect, label: &str, value: f32, focused: bool, active: bool) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let dial_color = if focused {
        if active {
            AMBER
        } else {
            ratatui::style::Color::Rgb(110, 80, 0)
        }
    } else if active {
        ratatui::style::Color::Rgb(100, 70, 0)
    } else {
        OFF
    };

    let dial_lines = build_dial(value, focused && active);
    let art: Vec<Line> = dial_lines
        .iter()
        .map(|l| Line::from(Span::styled(l.as_str(), Style::default().fg(dial_color))))
        .collect();
    f.render_widget(Paragraph::new(art).alignment(Alignment::Center), rows[0]);

    let bar_w = rows[1].width as usize;
    let filled = (value as f64 * bar_w as f64) as usize;
    let green_end = (bar_w as f64 * 0.60) as usize;
    let yellow_end = (bar_w as f64 * 0.85) as usize;

    let mut bar_spans = Vec::with_capacity(bar_w);
    for i in 0..bar_w {
        let (ch, color) = if i < filled {
            let c = if active {
                if i < green_end {
                    SAFE
                } else if i < yellow_end {
                    WARN
                } else {
                    HOT
                }
            } else if focused {
                ratatui::style::Color::Rgb(90, 65, 0)
            } else {
                OFF
            };
            ('█', c)
        } else {
            ('░', ratatui::style::Color::Rgb(30, 30, 30))
        };
        bar_spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
    }
    f.render_widget(Paragraph::new(Line::from(bar_spans)), rows[1]);

    let num = value * 10.0;
    let label_color = if focused {
        if active {
            AMBER
        } else {
            ratatui::style::Color::Rgb(110, 80, 0)
        }
    } else if active {
        DIM
    } else {
        OFF
    };
    let value_color = if focused {
        if active {
            ORANGE
        } else {
            ratatui::style::Color::Rgb(130, 90, 0)
        }
    } else if active {
        ratatui::style::Color::Rgb(120, 80, 0)
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
        rows[2],
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
        .style(Style::default().bg(ratatui::style::Color::Black));
    f.render_widget(help, area);
}

fn build_dial(value: f32, focused: bool) -> Vec<String> {
    use std::f32::consts::PI;

    let start_deg = 225.0_f32;
    let sweep = 270.0_f32;
    let angle = (start_deg - value * sweep) * PI / 180.0;

    // rx:ry = 2:1 compensates for terminal char aspect ratio (~2x taller than wide)
    let rx = 4.0_f32;
    let ry = 2.0_f32;
    let cx = 4.0_f32;
    let cy = 2.0_f32;

    let ix = (cx + angle.cos() * rx).round() as isize;
    let iy = (cy - angle.sin() * ry).round() as isize;

    let dot = if focused { '◆' } else { '◇' };

    (0..5isize)
        .map(|row| {
            (0..9isize)
                .map(|col| {
                    let dx = (col as f32 - cx) / rx;
                    let dy = (row as f32 - cy) / ry;
                    let dist = (dx * dx + dy * dy).sqrt();

                    if col == cx as isize && row == cy as isize {
                        '●'
                    } else if col == ix && row == iy {
                        dot
                    } else if (dist - 1.0).abs() < 0.25 {
                        let a = (-(row as f32 - cy)).atan2(col as f32 - cx).to_degrees();
                        // CCW distance from start: (start - a) mod 360
                        let rel = (start_deg - a).rem_euclid(360.0);
                        if rel <= sweep { '·' } else { ' ' }
                    } else {
                        ' '
                    }
                })
                .collect()
        })
        .collect()
}

fn amp_to_db(amp: f32) -> f32 {
    if amp < 1e-6 {
        -120.0
    } else {
        20.0 * amp.log10()
    }
}
