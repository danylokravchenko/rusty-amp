use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::preset::Preset;
use super::styles::{AMBER, CHROME, DIM, ORANGE};

pub(super) fn render_preset_modal(f: &mut Frame, presets: &[Preset], cursor: usize) {
    let area = centered_rect(60, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(
            " P R E S E T S ",
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ratatui::style::Color::Black));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::with_capacity(presets.len() + 1);

    let entry = |_idx: usize, name: &str, desc: &str, selected: bool| -> Line {
        let (prefix, name_style, desc_style) = if selected {
            (
                "▶ ",
                Style::default().fg(ORANGE).add_modifier(Modifier::BOLD | Modifier::REVERSED),
                Style::default().fg(AMBER).add_modifier(Modifier::REVERSED),
            )
        } else {
            ("  ", Style::default().fg(CHROME), Style::default().fg(DIM))
        };

        let label = if desc.is_empty() {
            name.to_string()
        } else {
            format!("{name}  ")
        };

        Line::from(vec![
            Span::styled(prefix.to_string(), Style::default().fg(if selected { ORANGE } else { DIM })),
            Span::styled(label, name_style),
            Span::styled(desc.to_string(), desc_style),
        ])
    };

    lines.push(entry(0, "Default values", "", cursor == 0));
    for (i, p) in presets.iter().enumerate() {
        let desc = p.description.as_deref().unwrap_or("");
        lines.push(entry(i + 1, &p.name, desc, cursor == i + 1));
    }

    let visible = rows[0].height as usize;
    let offset = if cursor >= visible { cursor - visible + 1 } else { 0 };
    let visible_lines: Vec<Line> = lines.into_iter().skip(offset).collect();

    f.render_widget(Paragraph::new(visible_lines), rows[0]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("↑/↓", Style::default().fg(AMBER)),
        Span::styled(" navigate  ", Style::default().fg(DIM)),
        Span::styled("Enter", Style::default().fg(AMBER)),
        Span::styled(" apply  ", Style::default().fg(DIM)),
        Span::styled("Esc / P", Style::default().fg(AMBER)),
        Span::styled(" close", Style::default().fg(DIM)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(footer, rows[1]);
}

fn centered_rect(percent_x: u16, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let width = area.width * percent_x / 100;
    let x = (area.width - width) / 2;

    let height = (area.height * 70 / 100).max(6);
    let y = (area.height - height) / 2;

    ratatui::layout::Rect {
        x: area.x + x,
        y: area.y + y,
        width,
        height,
    }
}
