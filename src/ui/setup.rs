use std::io::Stdout;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::audio::DeviceInfo;
use crate::dsp::{Levels, Params};

use super::draw::draw;
use super::styles::{AMBER, CHROME, DIM, ORANGE, WARM};

pub struct Selection {
    pub input_idx: usize,
    pub guitar_ch: usize,
    pub output_idx: usize,
}

enum Step {
    InputDevice {
        cursor: usize,
    },
    InputChannel {
        input_idx: usize,
        cursor: usize,
    },
    OutputDevice {
        input_idx: usize,
        guitar_ch: usize,
        cursor: usize,
    },
}

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    devices: &DeviceInfo,
    params: &Params,
    levels: &Levels,
) -> Result<Selection> {
    let mut step = Step::InputDevice { cursor: 0 };

    loop {
        let step_ref = &step;
        terminal.draw(|f| {
            draw(f, params, levels, None, &[], false, false, None, None, None);
            match step_ref {
                Step::InputDevice { cursor } => {
                    render_list_modal(
                        f,
                        " S E L E C T  I N P U T  D E V I C E ",
                        &devices
                            .inputs
                            .iter()
                            .map(|d| (d.name.as_str(), format!("{} ch", d.channels)))
                            .collect::<Vec<_>>(),
                        *cursor,
                        "↑/↓ navigate  Enter select  Ctrl-C quit",
                    );
                }
                Step::InputChannel { input_idx, cursor } => {
                    let n = devices.inputs[*input_idx].channels;
                    let items: Vec<(String, String)> = (1..=n)
                        .map(|i| (format!("Channel {i}"), String::new()))
                        .collect();
                    render_list_modal(
                        f,
                        " S E L E C T  G U I T A R  C H A N N E L ",
                        &items
                            .iter()
                            .map(|(a, b)| (a.as_str(), b.clone()))
                            .collect::<Vec<_>>(),
                        *cursor,
                        "↑/↓ navigate  Enter select  Ctrl-C quit",
                    );
                }
                Step::OutputDevice { cursor, .. } => {
                    let items: Vec<(&str, String)> = devices
                        .outputs
                        .iter()
                        .map(|name| (name.as_str(), String::new()))
                        .collect();
                    render_list_modal(
                        f,
                        " S E L E C T  O U T P U T  D E V I C E ",
                        &items,
                        *cursor,
                        "↑/↓ navigate  Enter select  Ctrl-C quit",
                    );
                }
            }
        })?;

        if !event::poll(Duration::from_millis(30))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Err(anyhow::anyhow!("quit"));
        }

        match &mut step {
            Step::InputDevice { cursor } => {
                let n = devices.inputs.len();
                match key.code {
                    KeyCode::Up => *cursor = cursor.saturating_sub(1),
                    KeyCode::Down => *cursor = (*cursor + 1).min(n.saturating_sub(1)),
                    KeyCode::Enter if n > 0 => {
                        let input_idx = *cursor;
                        step = Step::InputChannel {
                            input_idx,
                            cursor: 0,
                        };
                    }
                    _ => {}
                }
            }
            Step::InputChannel { input_idx, cursor } => {
                let n = devices.inputs[*input_idx].channels;
                match key.code {
                    KeyCode::Up => *cursor = cursor.saturating_sub(1),
                    KeyCode::Down => *cursor = (*cursor + 1).min(n.saturating_sub(1)),
                    KeyCode::Enter => {
                        let guitar_ch = *cursor;
                        step = Step::OutputDevice {
                            input_idx: *input_idx,
                            guitar_ch,
                            cursor: 0,
                        };
                    }
                    _ => {}
                }
            }
            Step::OutputDevice {
                input_idx,
                guitar_ch,
                cursor,
            } => {
                let n = devices.outputs.len();
                match key.code {
                    KeyCode::Up => *cursor = cursor.saturating_sub(1),
                    KeyCode::Down => *cursor = (*cursor + 1).min(n.saturating_sub(1)),
                    KeyCode::Enter if n > 0 => {
                        return Ok(Selection {
                            input_idx: *input_idx,
                            guitar_ch: *guitar_ch,
                            output_idx: *cursor,
                        });
                    }
                    _ => {}
                }
            }
        }
    }
}

fn render_list_modal(
    f: &mut ratatui::Frame,
    title: &str,
    items: &[(&str, String)],
    cursor: usize,
    footer_text: &str,
) {
    let area = centered_rect(62, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(
            title,
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ratatui::style::Color::Black));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let visible = rows[0].height as usize;
    let offset = if cursor >= visible {
        cursor - visible + 1
    } else {
        0
    };

    let lines: Vec<Line> = items
        .iter()
        .enumerate()
        .skip(offset)
        .map(|(i, (name, hint))| {
            let selected = i == cursor;
            let (prefix, name_style, hint_style) = if selected {
                (
                    "▶ ",
                    Style::default()
                        .fg(ORANGE)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED),
                    Style::default().fg(WARM).add_modifier(Modifier::REVERSED),
                )
            } else {
                ("  ", Style::default().fg(CHROME), Style::default().fg(DIM))
            };
            let label = if hint.is_empty() {
                name.to_string()
            } else {
                format!("{name}  ")
            };
            Line::from(vec![
                Span::styled(
                    prefix,
                    Style::default().fg(if selected { ORANGE } else { DIM }),
                ),
                Span::styled(label, name_style),
                Span::styled(hint.clone(), hint_style),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines), rows[0]);

    // Parse footer_text into alternating key/description spans
    let footer_spans: Vec<Span> = footer_text
        .split("  ")
        .enumerate()
        .flat_map(|(i, part)| {
            if let Some(pos) = part.find(' ') {
                let key = &part[..pos];
                let desc = &part[pos..];
                vec![
                    Span::styled(key.to_string(), Style::default().fg(AMBER)),
                    Span::styled(
                        if i == 0 {
                            desc.to_string()
                        } else {
                            format!("  {desc}")
                        },
                        Style::default().fg(DIM),
                    ),
                ]
            } else {
                vec![Span::styled(part.to_string(), Style::default().fg(AMBER))]
            }
        })
        .collect();

    f.render_widget(
        Paragraph::new(Line::from(footer_spans)).alignment(Alignment::Center),
        rows[1],
    );
}

fn centered_rect(percent_x: u16, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let width = area.width * percent_x / 100;
    let x = (area.width - width) / 2;
    let height = (area.height * 60 / 100).max(6);
    let y = (area.height - height) / 2;
    ratatui::layout::Rect {
        x: area.x + x,
        y: area.y + y,
        width,
        height,
    }
}
