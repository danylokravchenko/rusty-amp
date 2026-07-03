use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::dsp::metronome::{MAX_BPM, MIN_BPM, Metronome};

use super::styles::{AMBER, CHROME, DIM, HOT, ORANGE, SAFE};

pub(super) fn render_metronome(f: &mut Frame, metronome: &Metronome, blink: bool) {
    let area = centered_rect(60, 45, f.area());
    f.render_widget(Clear, area);

    let active = metronome.active.load(std::sync::atomic::Ordering::Relaxed);
    let bpm = metronome.get_bpm();

    let status_span = if active {
        Span::styled(" ● running — not recorded ", Style::default().fg(SAFE))
    } else {
        Span::styled(" ○ stopped ", Style::default().fg(DIM))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(
            " M E T R O N O M E ",
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(status_span).right_aligned())
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // gap
            Constraint::Length(1), // big BPM
            Constraint::Length(1), // "BPM" label
            Constraint::Length(1), // gap
            Constraint::Length(1), // slider
            Constraint::Length(1), // range labels
            Constraint::Length(1), // gap
            Constraint::Length(1), // beat indicator
            Constraint::Min(1),    // spacer
            Constraint::Length(1), // footer
        ])
        .split(inner);

    // Big tempo number, tinted when running.
    let bpm_color = if active { SAFE } else { CHROME };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {bpm}  "),
            Style::default()
                .fg(bpm_color)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )))
        .alignment(Alignment::Center),
        rows[1],
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled("BPM", Style::default().fg(DIM))))
            .alignment(Alignment::Center),
        rows[2],
    );

    render_slider(f, rows[4], bpm, active);

    // Range end labels beneath the slider.
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{MIN_BPM}"), Style::default().fg(DIM)),
            Span::styled("  slow ◀ ── tempo ── ▶ fast  ", Style::default().fg(DIM)),
            Span::styled(format!("{MAX_BPM}"), Style::default().fg(DIM)),
        ]))
        .alignment(Alignment::Center),
        rows[5],
    );

    // A blinking beat pip so the modal feels alive while it's running.
    let pip = if active && blink {
        Span::styled(
            "◆ tick",
            Style::default().fg(HOT).add_modifier(Modifier::BOLD),
        )
    } else if active {
        Span::styled("◇ tick", Style::default().fg(DIM))
    } else {
        Span::styled("— stopped —", Style::default().fg(DIM))
    };
    f.render_widget(
        Paragraph::new(Line::from(pip)).alignment(Alignment::Center),
        rows[7],
    );

    let toggle_hint = if active { "stop" } else { "start" };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("←/→", Style::default().fg(AMBER)),
            Span::styled(" tempo   ", Style::default().fg(DIM)),
            Span::styled("Space", Style::default().fg(AMBER)),
            Span::styled(format!(" {toggle_hint}   "), Style::default().fg(DIM)),
            Span::styled("Esc / M", Style::default().fg(AMBER)),
            Span::styled(" close", Style::default().fg(DIM)),
        ]))
        .alignment(Alignment::Center),
        rows[9],
    );
}

/// A horizontal tempo slider: a filled track with a marker at the current BPM.
fn render_slider(f: &mut Frame, area: Rect, bpm: u32, active: bool) {
    let w = area.width as usize;
    if w < 8 {
        return;
    }
    let track = w.saturating_sub(4);
    if track == 0 {
        return;
    }
    let frac = (bpm - MIN_BPM) as f32 / (MAX_BPM - MIN_BPM) as f32;
    let pos = (frac * (track - 1) as f32).round() as usize;
    let fill_color = if active { SAFE } else { ORANGE };

    let mut spans = vec![Span::styled("  ", Style::default())];
    for i in 0..track {
        let (ch, color) = if i == pos {
            ('▮', fill_color)
        } else if i < pos {
            ('─', fill_color)
        } else {
            ('·', DIM)
        };
        spans.push(Span::styled(
            ch.to_string(),
            Style::default().fg(color).add_modifier(if ch == '▮' {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ));
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let width = (area.width * percent_x / 100).max(40).min(area.width);
    let height = (area.height * percent_y / 100).max(12).min(area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    }
}
