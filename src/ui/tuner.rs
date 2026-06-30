use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::tuner::{SPECTRUM_BINS, Tuner, note_of, spectrum_bin_freq};

use super::styles::{AMBER, CHROME, DIM, HOT, ORANGE, SAFE, WARM, WARN};

/// Cents window (±) treated as "in tune".
const IN_TUNE_CENTS: f32 = 5.0;
/// Cents window (±) treated as "close" (amber); beyond this reads as out of tune.
const CLOSE_CENTS: f32 = 15.0;

/// Colour for a given cents offset: green in tune, amber close, red off.
fn tune_color(cents: f32) -> Color {
    let c = cents.abs();
    if c <= IN_TUNE_CENTS {
        SAFE
    } else if c <= CLOSE_CENTS {
        WARN
    } else {
        HOT
    }
}

pub(super) fn render_tuner(f: &mut Frame, tuner: &Tuner) {
    let area = centered_rect(70, 80, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(
            " T U N E R ",
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ))
        .title(
            Line::from(Span::styled(
                " rig bypassed — clean signal ",
                Style::default().fg(DIM),
            ))
            .right_aligned(),
        )
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // big note
            Constraint::Length(1), // cents meter
            Constraint::Length(1), // freq / status
            Constraint::Length(1), // gap
            Constraint::Min(3),    // spectrum
            Constraint::Length(1), // spectrum axis
            Constraint::Length(1), // footer
        ])
        .split(inner);

    let freq = tuner.freq.load(Relaxed);
    let has_pitch = freq > 0.0;
    let reading = has_pitch.then(|| note_of(freq));

    render_big_note(f, rows[0], reading.as_ref());
    render_cents_meter(f, rows[1], reading.as_ref().map(|r| r.cents));

    // Frequency / status line.
    let status = match &reading {
        Some(r) => {
            let color = tune_color(r.cents);
            let verdict = if r.cents.abs() <= IN_TUNE_CENTS {
                "IN TUNE"
            } else if r.cents < 0.0 {
                "TUNE UP ▲"
            } else {
                "TUNE DOWN ▼"
            };
            Line::from(vec![
                Span::styled(format!("{freq:7.2} Hz"), Style::default().fg(CHROME)),
                Span::styled("    ", Style::default()),
                Span::styled(format!("{:+.0} cents", r.cents), Style::default().fg(color)),
                Span::styled("    ", Style::default()),
                Span::styled(
                    verdict,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
            ])
        }
        None => Line::from(Span::styled(
            "— play a single note —",
            Style::default().fg(DIM),
        )),
    };
    f.render_widget(Paragraph::new(status).alignment(Alignment::Center), rows[2]);

    render_spectrum(f, rows[4], tuner, freq);
    render_axis(f, rows[5]);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Esc / T", Style::default().fg(AMBER)),
            Span::styled(" close", Style::default().fg(DIM)),
        ]))
        .alignment(Alignment::Center),
        rows[6],
    );
}

/// The detected note, rendered large and centred, coloured by tuning accuracy.
fn render_big_note(f: &mut Frame, area: Rect, reading: Option<&crate::dsp::tuner::NoteReading>) {
    let (text, color) = match reading {
        Some(r) => (format!("  {}{}  ", r.name, r.octave), tune_color(r.cents)),
        None => ("  --  ".to_string(), DIM),
    };

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            text,
            Style::default()
                .fg(color)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )),
        Line::from(""),
    ];
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

/// A horizontal needle meter: flat ◄ … centre … ► sharp. The marker sits left of
/// centre when flat, right when sharp, and turns green inside the in-tune window.
fn render_cents_meter(f: &mut Frame, area: Rect, cents: Option<f32>) {
    let w = area.width as usize;
    if w < 7 {
        return;
    }
    // Track between the flat/sharp end labels (2 cells each side).
    let track = w.saturating_sub(6);
    let center = track / 2;

    let mut cells = vec![('·', DIM); track];
    if track > 0 {
        cells[center] = ('│', CHROME);
    }

    let mut spans = vec![Span::styled("♭ ", Style::default().fg(DIM))];
    if let Some(c) = cents {
        let clamped = c.clamp(-50.0, 50.0);
        let pos = (center as f32 + clamped / 50.0 * center as f32)
            .round()
            .clamp(0.0, track as f32 - 1.0) as usize;
        let color = tune_color(c);
        cells[pos] = ('▮', color);
        for (ch, col) in &cells {
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(*col).add_modifier(if *ch == '▮' {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ));
        }
    } else {
        for (ch, col) in &cells {
            spans.push(Span::styled(ch.to_string(), Style::default().fg(*col)));
        }
    }
    spans.push(Span::styled(" ♯", Style::default().fg(DIM)));

    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

/// Vertical-bar magnitude spectrum across the display range. The bar nearest the
/// detected fundamental is highlighted so the played note stands out.
fn render_spectrum(f: &mut Frame, area: Rect, tuner: &Tuner, freq: f32) {
    let w = area.width as usize;
    let h = area.height as usize;
    if w == 0 || h == 0 {
        return;
    }

    // Snapshot the bars and find the bin nearest the detected fundamental.
    let mut mags = [0.0f32; SPECTRUM_BINS];
    for (m, cell) in mags.iter_mut().zip(&tuner.spectrum) {
        *m = cell.load(Relaxed);
    }
    let hot_bin = (freq > 0.0).then(|| {
        (0..SPECTRUM_BINS)
            .min_by(|&a, &b| {
                let da = (spectrum_bin_freq(a) / freq).ln().abs();
                let db = (spectrum_bin_freq(b) / freq).ln().abs();
                da.partial_cmp(&db).unwrap()
            })
            .unwrap()
    });

    // One value per terminal column, mapped from the bin bank.
    let col_bin = |x: usize| x * SPECTRUM_BINS / w;

    let mut lines: Vec<Line> = Vec::with_capacity(h);
    for row in 0..h {
        // Top row is the tallest threshold.
        let threshold = (h - row) as f32 / h as f32;
        let mut spans: Vec<Span> = Vec::with_capacity(w);
        for x in 0..w {
            let bin = col_bin(x);
            let v = mags[bin];
            let lit = v >= threshold;
            let highlight = hot_bin == Some(bin);
            let (ch, color) = if lit {
                let c = if highlight { SAFE } else { ORANGE };
                ('█', c)
            } else if highlight {
                // Keep the fundamental's column visible even where it's short.
                ('│', WARM)
            } else {
                (' ', DIM)
            };
            spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines), area);
}

/// A sparse Hz axis beneath the spectrum (low E … high register).
fn render_axis(f: &mut Frame, area: Rect) {
    let lo = spectrum_bin_freq(0);
    let hi = spectrum_bin_freq(SPECTRUM_BINS - 1);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{lo:.0} Hz"), Style::default().fg(DIM)),
            Span::styled(
                "  ◀ low ── note spectrum ── high ▶  ",
                Style::default().fg(DIM),
            ),
            Span::styled(format!("{hi:.0} Hz"), Style::default().fg(DIM)),
        ]))
        .alignment(Alignment::Center),
        area,
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let width = (area.width * percent_x / 100).max(40).min(area.width);
    let height = (area.height * percent_y / 100).max(16).min(area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    }
}
