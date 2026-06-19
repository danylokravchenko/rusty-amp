use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use atomic_float::AtomicF32;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::{AmpModel, Levels, Params};

// ── Palette ───────────────────────────────────────────────────────────────────

const ORANGE: Color = Color::Rgb(255, 140, 0);
const AMBER:  Color = Color::Rgb(255, 200, 60);
const DIM:    Color = Color::Rgb(80, 80, 80);
const CHROME: Color = Color::Rgb(180, 180, 190);
const HOT:    Color = Color::Rgb(220, 30, 30);
const WARM:   Color = Color::Rgb(200, 100, 0);
const SAFE:   Color = Color::Rgb(40, 180, 40);
const WARN:   Color = Color::Rgb(220, 180, 0);
const OFF:    Color = Color::Rgb(50, 50, 50);

// ── Knob descriptors ──────────────────────────────────────────────────────────

struct Knob {
    label: &'static str,
    param: fn(&Params) -> &Arc<AtomicF32>,
}

// Knob index layout — order matches left-to-right visual navigation:
//   0-2 : TS-808           (pedals row, left)
//   3-5 : DS-1 Distortion  (pedals row, centre)
//   6-8 : Spring Reverb    (pedals row, right)
//   9-13: Amp              (amp row, full width)
const KNOBS: &[Knob] = &[
    Knob { label: "DRIVE", param: |p| &p.ts_drive  },
    Knob { label: "TONE",  param: |p| &p.ts_tone   },
    Knob { label: "LEVEL", param: |p| &p.ts_level  },

    Knob { label: "DRIVE", param: |p| &p.ds_drive  },
    Knob { label: "TONE",  param: |p| &p.ds_tone   },
    Knob { label: "LEVEL", param: |p| &p.ds_level  },

    Knob { label: "ROOM", param: |p| &p.rev_room },
    Knob { label: "DAMP", param: |p| &p.rev_damp },
    Knob { label: "MIX",  param: |p| &p.rev_mix  },

    Knob { label: "GAIN",   param: |p| &p.amp_gain   },
    Knob { label: "BASS",   param: |p| &p.amp_bass   },
    Knob { label: "MID",    param: |p| &p.amp_mid    },
    Knob { label: "TREBLE", param: |p| &p.amp_treble },
    Knob { label: "MASTER", param: |p| &p.amp_master },
];

// (title_fn, start, end, enabled_fn)  — enabled_fn returns None for non-toggleable sections
type SectionDef = (fn(&Params) -> String, usize, usize, fn(&Params) -> Option<bool>);

const PEDAL_SECTIONS: &[SectionDef] = &[
    (
        |_| "⚡ TS-808".into(),
        0, 3,
        |p| Some(p.ts_enabled.load(Relaxed)),
    ),
    (
        |_| "⚡ DS-1 DISTORTION".into(),
        3, 6,
        |p| Some(p.ds_enabled.load(Relaxed)),
    ),
    (
        |_| "⚡ SPRING REVERB".into(),
        6, 9,
        |p| Some(p.rev_enabled.load(Relaxed)),
    ),
];

const AMP_SECTIONS: &[SectionDef] = &[
    (
        |p| format!("⚡ {}", p.amp_model().name()),
        9, 14,
        |_| None,
    ),
];

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(params: Arc<Params>, levels: Arc<Levels>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Focus: None = amp model selector row, Some(i) = knob i
    let mut focus: Option<usize> = None;

    loop {
        terminal.draw(|f| draw(f, &params, &levels, focus))?;

        if event::poll(Duration::from_millis(30))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,

                    // Tab / Shift-Tab → jump between sections
                    KeyCode::Tab => focus = next_section(focus),
                    KeyCode::BackTab => focus = prev_section(focus),

                    // Left / Right → move one knob at a time
                    KeyCode::Right => {
                        focus = match focus {
                            None    => Some(0),
                            Some(i) => Some((i + 1) % KNOBS.len()),
                        };
                    }
                    KeyCode::Left => {
                        focus = match focus {
                            None    => Some(KNOBS.len() - 1),
                            Some(0) => None,
                            Some(i) => Some(i - 1),
                        };
                    }

                    KeyCode::Up | KeyCode::Char('+') | KeyCode::Char('=') => {
                        match focus {
                            None    => cycle_amp(&params, 1),
                            Some(i) => nudge(&params, i, 0.05),
                        }
                    }
                    KeyCode::Down | KeyCode::Char('-') => {
                        match focus {
                            None    => cycle_amp(&params, -1),
                            Some(i) => nudge(&params, i, -0.05),
                        }
                    }

                    // Space toggles the pedal whose section contains the focused knob
                    KeyCode::Char(' ') => {
                        if let Some(i) = focus {
                            toggle_pedal(&params, i);
                        }
                    }

                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

// Section start knob indices in Tab order (None = amp model selector).
// Must match the visual layout: selector → TS-808 → DS-1 → Reverb → Amp.
const SECTION_STARTS: &[Option<usize>] = &[None, Some(0), Some(3), Some(6), Some(9)];

fn section_of(focus: Option<usize>) -> usize {
    match focus {
        None       => 0,
        Some(i) if i < 3  => 1,
        Some(i) if i < 6  => 2,
        Some(i) if i < 9  => 3,
        Some(_)            => 4,
    }
}

fn next_section(focus: Option<usize>) -> Option<usize> {
    let next = (section_of(focus) + 1) % SECTION_STARTS.len();
    SECTION_STARTS[next]
}

fn prev_section(focus: Option<usize>) -> Option<usize> {
    let cur = section_of(focus);
    let prev = (cur + SECTION_STARTS.len() - 1) % SECTION_STARTS.len();
    SECTION_STARTS[prev]
}

fn nudge(params: &Params, idx: usize, delta: f32) {
    let atom = (KNOBS[idx].param)(params);
    let new = (atom.load(Relaxed) + delta).clamp(0.0, 1.0);
    atom.store(new, Relaxed);
}

fn cycle_amp(params: &Params, dir: i8) {
    let current = AmpModel::from_u8(params.amp_model.load(Relaxed));
    let next = if dir >= 0 { current.next() } else { current.prev() };
    params.amp_model.store(next as u8, Relaxed);
}

fn toggle_pedal(params: &Params, knob_idx: usize) {
    if knob_idx < 3 {
        let v = params.ts_enabled.load(Relaxed);
        params.ts_enabled.store(!v, Relaxed);
    } else if knob_idx < 6 {
        let v = params.ds_enabled.load(Relaxed);
        params.ds_enabled.store(!v, Relaxed);
    } else if knob_idx < 9 {
        let v = params.rev_enabled.load(Relaxed);
        params.rev_enabled.store(!v, Relaxed);
    }
    // knob_idx 9-13 → amp, not toggleable
}

// ── Top-level layout ──────────────────────────────────────────────────────────

fn draw(f: &mut Frame, params: &Params, levels: &Levels, focus: Option<usize>) {
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
            Constraint::Length(5), // VU meters
            Constraint::Length(3), // amp model selector
            Constraint::Ratio(1, 2), // pedals row
            Constraint::Ratio(1, 2), // amp + reverb row
            Constraint::Length(1), // help bar
        ])
        .split(inner);

    render_header(f, rows[0], params);
    render_meters(f, rows[1], levels);
    render_amp_selector(f, rows[2], params, focus.is_none());
    render_section_row(f, rows[3], PEDAL_SECTIONS, params, focus);
    render_section_row(f, rows[4], AMP_SECTIONS, params, focus);
    render_help(f, rows[5]);
}

// ── Header ────────────────────────────────────────────────────────────────────

fn render_header(f: &mut Frame, area: Rect, params: &Params) {
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
    ]));
    f.render_widget(title, rows[0]);

    let ts_on  = params.ts_enabled .load(Relaxed);
    let ds_on  = params.ds_enabled .load(Relaxed);
    let rev_on = params.rev_enabled.load(Relaxed);

    let arrow = Span::styled(" ──▶ ", Style::default().fg(DIM));

    let pedal_color = |on: bool| if on { ORANGE } else { OFF };
    let amp_color   = AMBER;

    let mut chain: Vec<Span> = vec![Span::raw("  ")];

    chain.push(Span::styled("TS-808",       Style::default().fg(pedal_color(ts_on))));
    chain.push(arrow.clone());
    chain.push(Span::styled("DS-1",         Style::default().fg(pedal_color(ds_on))));
    chain.push(arrow.clone());
    chain.push(Span::styled("PREAMP",       Style::default().fg(amp_color)));
    chain.push(arrow.clone());
    chain.push(Span::styled("TONE STACK",   Style::default().fg(amp_color)));
    chain.push(arrow.clone());
    chain.push(Span::styled("POWER AMP",    Style::default().fg(amp_color)));
    chain.push(arrow.clone());
    chain.push(Span::styled("REVERB",       Style::default().fg(pedal_color(rev_on))));
    chain.push(arrow.clone());
    chain.push(Span::styled("OUTPUT",       Style::default().fg(CHROME)));

    f.render_widget(Paragraph::new(Line::from(chain)), rows[1]);
}

// ── VU Meters ─────────────────────────────────────────────────────────────────

fn render_meters(f: &mut Frame, area: Rect, levels: &Levels) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(WARM))
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

    let in_level  = levels.input .load(Relaxed);
    let out_level = levels.output.load(Relaxed);
    let out_db    = amp_to_db(out_level);
    let watts     = (out_level * out_level * 100.0).min(100.0);

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
                .fg(if watts > 80.0 { HOT } else if watts > 40.0 { WARN } else { SAFE })
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
    let db   = amp_to_db(level);
    let fill = ((db + 60.0) / 60.0).clamp(0.0, 1.0) as f64;

    let bar_width = (area.width as usize).saturating_sub(label.len() + 2 + 10);
    let filled    = (fill * bar_width as f64) as usize;

    let green_end  = (bar_width as f64 * 0.72) as usize;
    let yellow_end = (bar_width as f64 * 0.88) as usize;

    let mut spans: Vec<Span> = vec![
        Span::styled(label, Style::default().fg(CHROME).add_modifier(Modifier::BOLD)),
        Span::styled("▐", Style::default().fg(DIM)),
    ];

    for i in 0..bar_width {
        let ch    = if i < filled { '█' } else { '░' };
        let color = if i < filled {
            if i < green_end { SAFE } else if i < yellow_end { WARN } else { HOT }
        } else {
            Color::Rgb(30, 30, 30)
        };
        spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
    }
    spans.push(Span::styled("▌", Style::default().fg(DIM)));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ── Amp model selector ────────────────────────────────────────────────────────

fn render_amp_selector(f: &mut Frame, area: Rect, params: &Params, focused: bool) {
    let border_color = if focused { ORANGE } else { Color::Rgb(60, 40, 0) };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let model = params.amp_model();

    let mut spans: Vec<Span> = vec![
        Span::styled(
            "  AMP MODEL  ",
            Style::default()
                .fg(if focused { AMBER } else { DIM })
                .add_modifier(Modifier::BOLD),
        ),
    ];

    for m in [AmpModel::Marshall, AmpModel::Mesa, AmpModel::Randall] {
        let selected = m == model;
        let label_style = if selected {
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(Color::Rgb(80, 60, 0))
        };
        let (bl, br) = if selected { ("◀ ", " ▶") } else { ("[ ", " ]") };
        let bracket_color = if selected { AMBER } else { DIM };
        spans.push(Span::styled(bl, Style::default().fg(bracket_color)));
        spans.push(Span::styled(m.short_name(), label_style));
        spans.push(Span::styled(br, Style::default().fg(bracket_color)));
        spans.push(Span::raw("  "));
    }

    if focused {
        spans.push(Span::styled("  ↑/↓ to switch", Style::default().fg(DIM)));
    }

    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Left),
        inner,
    );
}

// ── Section rows ──────────────────────────────────────────────────────────────

fn render_section_row(
    f: &mut Frame,
    area: Rect,
    sections: &[SectionDef],
    params: &Params,
    focus: Option<usize>,
) {
    let n = sections.len() as u32;
    let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Ratio(1, n)).collect();

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (i, (title_fn, start, end, enabled_fn)) in sections.iter().enumerate() {
        let title   = title_fn(params);
        let enabled = enabled_fn(params);
        render_section(f, cols[i], &title, params, *start, *end, enabled, focus);
    }
}

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
    let active       = focus.map_or(false, |f| f >= start && f < end);
    let is_on        = enabled.unwrap_or(true); // non-toggleable sections are always "on"

    let border_color = if active {
        ORANGE                     // focus always visible, even when pedal is off
    } else if !is_on {
        Color::Rgb(40, 30, 0)      // dim — pedal off, not focused
    } else {
        Color::Rgb(60, 40, 0)
    };

    // Build title with optional ON/OFF badge
    let title_text = match enabled {
        Some(true)  => format!(" {title} ● "),
        Some(false) => format!(" {title} ○ "),
        None        => format!(" {title} "),
    };
    let title_color = if !is_on {
        OFF
    } else if active {
        AMBER
    } else {
        DIM
    };
    let badge_color = if !is_on { OFF } else if active { SAFE } else { Color::Rgb(0, 100, 0) };

    // Split title and badge for separate styling
    let title_spans = if let Some(on) = enabled {
        vec![
            Span::styled(
                format!(" {title} "),
                Style::default().fg(title_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if on { "● " } else { "○ " },
                Style::default().fg(badge_color).add_modifier(Modifier::BOLD),
            ),
        ]
    } else {
        vec![Span::styled(
            title_text,
            Style::default().fg(title_color).add_modifier(Modifier::BOLD),
        )]
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(title_spans))
        .style(Style::default().bg(Color::Black));

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
        if active { AMBER } else { Color::Rgb(110, 80, 0) }  // muted amber — focused but pedal off
    } else if active {
        Color::Rgb(100, 70, 0)
    } else {
        OFF
    };

    let dial_lines = build_dial(value, focused && active);
    let art: Vec<Line> = dial_lines
        .iter()
        .map(|l| Line::from(Span::styled(l.as_str(), Style::default().fg(dial_color))))
        .collect();
    f.render_widget(Paragraph::new(art).alignment(Alignment::Center), rows[0]);

    // Gauge bar
    let bar_w  = rows[1].width as usize;
    let filled = (value as f64 * bar_w as f64) as usize;
    let green_end  = (bar_w as f64 * 0.60) as usize;
    let yellow_end = (bar_w as f64 * 0.85) as usize;

    let mut bar_spans: Vec<Span> = Vec::with_capacity(bar_w);
    for i in 0..bar_w {
        let (ch, color) = if i < filled {
            let c = if active {
                if i < green_end { SAFE } else if i < yellow_end { WARN } else { HOT }
            } else if focused {
                Color::Rgb(90, 65, 0)  // muted fill — focused but pedal off
            } else {
                OFF
            };
            ('█', c)
        } else {
            ('░', Color::Rgb(30, 30, 30))
        };
        bar_spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
    }
    f.render_widget(Paragraph::new(Line::from(bar_spans)), rows[1]);

    // Label + value
    let num         = value * 10.0;
    let label_color = if focused {
        if active { AMBER } else { Color::Rgb(110, 80, 0) }
    } else if active {
        DIM
    } else {
        OFF
    };
    let value_color = if focused {
        if active { ORANGE } else { Color::Rgb(130, 90, 0) }
    } else if active {
        Color::Rgb(120, 80, 0)
    } else {
        OFF
    };

    let label_line = Line::from(vec![
        Span::styled(
            format!("{label} "),
            Style::default().fg(label_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{num:.1}"),
            Style::default().fg(value_color).add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(
        Paragraph::new(label_line).alignment(Alignment::Center),
        rows[2],
    );
}

// ── Help bar ──────────────────────────────────────────────────────────────────

fn render_help(f: &mut Frame, area: Rect) {
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Tab ", Style::default().fg(AMBER)),
        Span::styled("section  ", Style::default().fg(DIM)),
        Span::styled("←/→", Style::default().fg(AMBER)),
        Span::styled(" knob  ", Style::default().fg(DIM)),
        Span::styled("↑/↓  +/-", Style::default().fg(AMBER)),
        Span::styled(" adjust / switch amp  ", Style::default().fg(DIM)),
        Span::styled("Space", Style::default().fg(AMBER)),
        Span::styled(" toggle pedal  ", Style::default().fg(DIM)),
        Span::styled("Q", Style::default().fg(AMBER)),
        Span::styled(" quit", Style::default().fg(DIM)),
    ]))
    .alignment(Alignment::Center)
    .style(Style::default().bg(Color::Black));
    f.render_widget(help, area);
}

// ── ASCII dial ────────────────────────────────────────────────────────────────

fn build_dial(value: f32, focused: bool) -> Vec<String> {
    use std::f32::consts::PI;

    let start = 225.0_f32;
    let sweep = 300.0_f32;
    let angle = (start - value * sweep) * PI / 180.0;

    let r  = 2.0_f32;
    let cx = 4.0_f32;
    let cy = 2.0_f32;
    let ix = (cx + angle.cos() * r).round() as isize;
    let iy = (cy - angle.sin() * r).round() as isize;

    let dot = if focused { '◆' } else { '◇' };

    (0..5isize)
        .map(|row| {
            (0..9isize)
                .map(|col| {
                    let dc   = (col as f32 - cx) as isize;
                    let dr   = (row as f32 - cy) as isize;
                    let dist = ((dc * dc + dr * dr) as f32).sqrt();

                    if col == cx as isize && row == cy as isize {
                        '●'
                    } else if col == ix && row == iy {
                        dot
                    } else if (dist - r).abs() < 0.55 {
                        let a   = (-(row as f32 - cy)).atan2(col as f32 - cx).to_degrees();
                        let rel = (a - start).rem_euclid(360.0);
                        if rel <= sweep { '·' } else { ' ' }
                    } else {
                        ' '
                    }
                })
                .collect()
        })
        .collect()
}

// ── Utilities ─────────────────────────────────────────────────────────────────

fn amp_to_db(amp: f32) -> f32 {
    if amp < 1e-6 { -120.0 } else { 20.0 * amp.log10() }
}
