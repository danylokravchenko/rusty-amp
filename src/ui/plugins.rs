//! Plugin browser modal (behind the `clap` feature): scan, load, and clear a
//! third-party CLAP effect plugin used as the chain's stereo insert.

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use super::styles::{AMBER, CHROME, DIM, ORANGE, SAFE};
use crate::audio::AudioEngine;
use crate::host::{self, DiscoveredPlugin, LoadedPlugin};

/// All plugin-browser state, kept on the UI thread.
pub(super) struct PluginBrowser {
    /// Whether the modal is currently shown.
    pub(super) open: bool,
    cursor: usize,
    plugins: Vec<DiscoveredPlugin>,
    /// The currently loaded plugin's main-thread handle, kept alive while in use.
    loaded: Option<LoadedPlugin>,
    /// Last action result, surfaced in the modal footer.
    message: Option<String>,
    sample_rate: f32,
    max_block: u32,
}

impl PluginBrowser {
    pub(super) fn new(sample_rate: f32, max_block: u32) -> Self {
        Self {
            open: false,
            cursor: 0,
            plugins: Vec::new(),
            loaded: None,
            message: None,
            sample_rate,
            max_block,
        }
    }

    /// Open the modal, (re)scanning the standard CLAP locations.
    pub(super) fn open(&mut self) {
        self.plugins = host::scan();
        self.cursor = 0;
        self.open = true;
    }

    /// Name of the loaded plugin, for the main status line.
    pub(super) fn loaded_name(&self) -> Option<&str> {
        self.loaded.as_ref().map(|p| p.name.as_str())
    }

    /// Handle a keypress while the modal is open. Loading/clearing is applied to
    /// the live `engine` immediately.
    pub(super) fn handle_key(&mut self, code: KeyCode, engine: &mut AudioEngine) {
        // Entry 0 is the "clear" row; entries 1.. map to `self.plugins`.
        let total = self.plugins.len() + 1;
        match code {
            KeyCode::Up => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Down => self.cursor = (self.cursor + 1).min(total - 1),
            KeyCode::Enter => self.activate_selection(engine),
            KeyCode::Esc | KeyCode::Char('v') | KeyCode::Char('V') => self.open = false,
            _ => {}
        }
    }

    fn activate_selection(&mut self, engine: &mut AudioEngine) {
        if self.cursor == 0 {
            self.message = match engine.set_plugin_insert(None) {
                Ok(()) => {
                    self.loaded = None;
                    Some("Insert cleared".to_owned())
                }
                Err(e) => Some(format!("Clear failed: {e}")),
            };
            self.open = false;
            return;
        }

        let Some(plugin) = self.plugins.get(self.cursor - 1) else {
            return;
        };

        self.message = match host::load(plugin, self.sample_rate, self.max_block) {
            Ok((loaded, insert)) => match engine.set_plugin_insert(Some(insert)) {
                Ok(()) => {
                    let name = loaded.name.clone();
                    // Replacing keeps the old handle alive only until here; the audio
                    // thread already holds the processor's own ref, so this is safe.
                    self.loaded = Some(loaded);
                    Some(format!("Loaded {name}"))
                }
                Err(e) => Some(format!("Load failed: {e}")),
            },
            Err(e) => Some(format!("Load failed: {e}")),
        };
        self.open = false;
    }

    /// Render the modal over the main UI.
    pub(super) fn render(&self, f: &mut Frame) {
        let area = centered_rect(60, f.area());
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(ORANGE))
            .title(Span::styled(
                " C L A P   P L U G I N S ",
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(inner);

        let mut lines: Vec<Line> = Vec::with_capacity(self.plugins.len() + 1);
        lines.push(entry("None — bypass insert", "", self.cursor == 0));
        for (i, p) in self.plugins.iter().enumerate() {
            lines.push(entry(&p.name, &p.id, self.cursor == i + 1));
        }
        if self.plugins.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (no CLAP plugins found in the standard locations)",
                Style::default().fg(DIM),
            )));
        }

        let visible = rows[0].height as usize;
        let offset = self.cursor.saturating_sub(visible.saturating_sub(1));
        let visible_lines: Vec<Line> = lines.into_iter().skip(offset).collect();
        f.render_widget(Paragraph::new(visible_lines), rows[0]);

        let footer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(rows[1]);

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("↑/↓", Style::default().fg(AMBER)),
                Span::styled(" navigate  ", Style::default().fg(DIM)),
                Span::styled("Enter", Style::default().fg(AMBER)),
                Span::styled(" load/clear  ", Style::default().fg(DIM)),
                Span::styled("Esc / V", Style::default().fg(AMBER)),
                Span::styled(" close", Style::default().fg(DIM)),
            ]))
            .alignment(Alignment::Center),
            footer[0],
        );

        let status_line = match (&self.message, self.loaded_name()) {
            (Some(msg), _) => Line::from(Span::styled(
                msg.clone(),
                Style::default().fg(SAFE).add_modifier(Modifier::BOLD),
            )),
            (None, Some(name)) => Line::from(vec![
                Span::styled("active: ", Style::default().fg(DIM)),
                Span::styled(name.to_owned(), Style::default().fg(CHROME)),
            ]),
            (None, None) => Line::from(Span::styled(
                "no plugin loaded",
                Style::default().fg(DIM),
            )),
        };
        f.render_widget(Paragraph::new(status_line).alignment(Alignment::Center), footer[1]);
    }
}

fn entry(name: &str, detail: &str, selected: bool) -> Line<'static> {
    let (prefix, name_style) = if selected {
        (
            "▶ ",
            Style::default()
                .fg(ORANGE)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
    } else {
        ("  ", Style::default().fg(CHROME))
    };
    let mut spans = vec![
        Span::styled(
            prefix.to_owned(),
            Style::default().fg(if selected { ORANGE } else { DIM }),
        ),
        Span::styled(name.to_owned(), name_style),
    ];
    if !detail.is_empty() {
        spans.push(Span::styled(format!("  {detail}"), Style::default().fg(DIM)));
    }
    Line::from(spans)
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
