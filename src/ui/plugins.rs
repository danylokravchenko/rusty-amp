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

/// Which page of the modal is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    /// Pick a plugin to load (or clear the insert).
    Browse,
    /// Edit the loaded plugin's parameters.
    Edit,
}

/// All plugin-browser state, kept on the UI thread.
pub(super) struct PluginBrowser {
    /// Whether the modal is currently shown.
    pub(super) open: bool,
    view: View,
    cursor: usize,
    /// Selected parameter in the Edit view.
    param_cursor: usize,
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
            view: View::Browse,
            cursor: 0,
            param_cursor: 0,
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
        self.view = View::Browse;
        self.open = true;
    }

    /// Name of the loaded plugin, for the main status line.
    pub(super) fn loaded_name(&self) -> Option<&str> {
        self.loaded.as_ref().map(|p| p.name.as_str())
    }

    /// Handle a keypress while the modal is open. Loading/clearing and parameter
    /// edits are applied to the live `engine`/plugin immediately.
    pub(super) fn handle_key(&mut self, code: KeyCode, engine: &mut AudioEngine) {
        match self.view {
            View::Browse => self.handle_browse_key(code, engine),
            View::Edit => self.handle_edit_key(code),
        }
    }

    fn handle_browse_key(&mut self, code: KeyCode, engine: &mut AudioEngine) {
        // Entry 0 is the "clear" row; entries 1.. map to `self.plugins`.
        let total = self.plugins.len() + 1;
        match code {
            KeyCode::Up => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Down => self.cursor = (self.cursor + 1).min(total - 1),
            KeyCode::Enter => self.activate_selection(engine),
            // Switch to the parameter editor (only useful with a plugin loaded).
            KeyCode::Tab if self.loaded.is_some() => {
                self.view = View::Edit;
                self.param_cursor = 0;
            }
            KeyCode::Esc | KeyCode::Char('v') | KeyCode::Char('V') => self.open = false,
            _ => {}
        }
    }

    fn handle_edit_key(&mut self, code: KeyCode) {
        let Some(loaded) = self.loaded.as_mut() else {
            self.view = View::Browse;
            return;
        };
        let count = loaded.params().len();
        match code {
            KeyCode::Up => self.param_cursor = self.param_cursor.saturating_sub(1),
            KeyCode::Down if count > 0 => {
                self.param_cursor = (self.param_cursor + 1).min(count - 1);
            }
            KeyCode::Left | KeyCode::Char('-') => self.nudge_param(-1),
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => self.nudge_param(1),
            KeyCode::Tab => self.view = View::Browse,
            KeyCode::Esc | KeyCode::Char('v') | KeyCode::Char('V') => self.open = false,
            _ => {}
        }
    }

    /// Adjust the selected parameter by one step in `dir` (±1), 1/20 of its range.
    fn nudge_param(&mut self, dir: i32) {
        let Some(loaded) = self.loaded.as_mut() else {
            return;
        };
        let Some(param) = loaded.params().get(self.param_cursor) else {
            return;
        };
        let step = (param.max - param.min) / 20.0;
        let target = param.value + f64::from(dir) * step;
        loaded.set_param(self.param_cursor, target);
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
                    let has_params = !loaded.params().is_empty();
                    // Replacing keeps the old handle alive only until here; the audio
                    // thread already holds the processor's own ref, so this is safe.
                    self.loaded = Some(loaded);
                    // Jump straight into the parameter editor if there's anything to edit.
                    if has_params {
                        self.view = View::Edit;
                        self.param_cursor = 0;
                    } else {
                        self.open = false;
                    }
                    Some(format!("Loaded {name}"))
                }
                Err(e) => Some(format!("Load failed: {e}")),
            },
            Err(e) => Some(format!("Load failed: {e}")),
        };
    }

    /// Render the modal over the main UI.
    pub(super) fn render(&self, f: &mut Frame) {
        let area = centered_rect(60, f.area());
        f.render_widget(Clear, area);

        let title = match self.view {
            View::Browse => " C L A P   P L U G I N S ",
            View::Edit => " P L U G I N   P A R A M S ",
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(ORANGE))
            .title(Span::styled(
                title,
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(inner);

        match self.view {
            View::Browse => self.render_browse(f, rows[0]),
            View::Edit => self.render_edit(f, rows[0]),
        }

        let footer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(rows[1]);

        let hint = match self.view {
            View::Browse => vec![
                Span::styled("↑/↓", Style::default().fg(AMBER)),
                Span::styled(" navigate  ", Style::default().fg(DIM)),
                Span::styled("Enter", Style::default().fg(AMBER)),
                Span::styled(" load/clear  ", Style::default().fg(DIM)),
                Span::styled("Tab", Style::default().fg(AMBER)),
                Span::styled(" params  ", Style::default().fg(DIM)),
                Span::styled("Esc / V", Style::default().fg(AMBER)),
                Span::styled(" close", Style::default().fg(DIM)),
            ],
            View::Edit => vec![
                Span::styled("↑/↓", Style::default().fg(AMBER)),
                Span::styled(" select  ", Style::default().fg(DIM)),
                Span::styled("←/→", Style::default().fg(AMBER)),
                Span::styled(" adjust  ", Style::default().fg(DIM)),
                Span::styled("Tab", Style::default().fg(AMBER)),
                Span::styled(" browse  ", Style::default().fg(DIM)),
                Span::styled("Esc / V", Style::default().fg(AMBER)),
                Span::styled(" close", Style::default().fg(DIM)),
            ],
        };
        f.render_widget(
            Paragraph::new(Line::from(hint)).alignment(Alignment::Center),
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
            (None, None) => Line::from(Span::styled("no plugin loaded", Style::default().fg(DIM))),
        };
        f.render_widget(
            Paragraph::new(status_line).alignment(Alignment::Center),
            footer[1],
        );
    }

    fn render_browse(&self, f: &mut Frame, area: ratatui::layout::Rect) {
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

        let visible = area.height as usize;
        let offset = self.cursor.saturating_sub(visible.saturating_sub(1));
        let visible_lines: Vec<Line> = lines.into_iter().skip(offset).collect();
        f.render_widget(Paragraph::new(visible_lines), area);
    }

    fn render_edit(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let Some(loaded) = self.loaded.as_ref() else {
            return;
        };
        let params = loaded.params();
        if params.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "  (this plugin exposes no parameters)",
                    Style::default().fg(DIM),
                )),
                area,
            );
            return;
        }

        let lines: Vec<Line> = params
            .iter()
            .enumerate()
            .map(|(i, p)| param_row(p, i == self.param_cursor))
            .collect();

        let visible = area.height as usize;
        let offset = self.param_cursor.saturating_sub(visible.saturating_sub(1));
        let visible_lines: Vec<Line> = lines.into_iter().skip(offset).collect();
        f.render_widget(Paragraph::new(visible_lines), area);
    }
}

/// Render one parameter row: name, a fill bar, and the current value.
fn param_row(p: &crate::host::PluginParam, selected: bool) -> Line<'static> {
    let span = (p.max - p.min).abs();
    let fill = if span > f64::EPSILON {
        ((p.value - p.min) / span).clamp(0.0, 1.0)
    } else {
        0.0
    };
    const BAR: usize = 16;
    let filled = (fill * BAR as f64).round() as usize;
    let bar: String = (0..BAR)
        .map(|i| if i < filled { '█' } else { '░' })
        .collect();

    let (prefix, name_style, bar_color) = if selected {
        (
            "▶ ",
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
            ORANGE,
        )
    } else {
        ("  ", Style::default().fg(CHROME), DIM)
    };

    Line::from(vec![
        Span::styled(
            prefix,
            Style::default().fg(if selected { ORANGE } else { DIM }),
        ),
        Span::styled(format!("{:<22}", truncate(&p.name, 22)), name_style),
        Span::styled(bar, Style::default().fg(bar_color)),
        Span::styled(format!("  {:.3}", p.value), Style::default().fg(AMBER)),
    ])
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
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
        spans.push(Span::styled(
            format!("  {detail}"),
            Style::default().fg(DIM),
        ));
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
