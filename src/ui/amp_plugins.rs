//! Amp-plugin browser modal (behind the `au` feature, macOS only): scan, load, and
//! clear an Audio Unit used as the chain's amp-position override. Loading an AU makes
//! it active (built-in amp+cab bypassed); it can also be toggled live from the main UI.

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use std::sync::atomic::Ordering::Relaxed;

use super::styles::{AMBER, CHROME, DIM, ORANGE, SAFE};
use crate::audio::AudioEngine;
use crate::dsp::Params;
use crate::host::au::{self, AuParam, DiscoveredAu, LoadedAu};

/// Which page of the modal is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    /// Pick an AU to load (or clear the override).
    Browse,
    /// Edit the loaded AU's parameters.
    Edit,
}

/// All amp-plugin browser state, kept on the UI thread.
pub(super) struct AmpBrowser {
    pub(super) open: bool,
    view: View,
    cursor: usize,
    param_cursor: usize,
    plugins: Vec<DiscoveredAu>,
    /// The currently loaded AU's UI-side handle, kept alive while in use.
    loaded: Option<LoadedAu>,
    message: Option<String>,
    sample_rate: f32,
    max_block: u32,
}

impl AmpBrowser {
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

    /// Open the modal, (re)scanning the installed Audio Units.
    pub(super) fn open(&mut self) {
        self.plugins = au::scan();
        self.cursor = 0;
        self.view = View::Browse;
        self.open = true;
    }

    /// Name of the loaded AU, for the main status line.
    pub(super) fn loaded_name(&self) -> Option<&str> {
        self.loaded.as_ref().map(|p| p.name.as_str())
    }

    pub(super) fn handle_key(&mut self, code: KeyCode, engine: &mut AudioEngine, params: &Params) {
        match self.view {
            View::Browse => self.handle_browse_key(code, engine, params),
            View::Edit => self.handle_edit_key(code),
        }
    }

    fn handle_browse_key(&mut self, code: KeyCode, engine: &mut AudioEngine, params: &Params) {
        // Entry 0 is the "clear" row; entries 1.. map to `self.plugins`.
        let total = self.plugins.len() + 1;
        match code {
            KeyCode::Up => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Down => self.cursor = (self.cursor + 1).min(total - 1),
            KeyCode::Enter => self.activate_selection(engine, params),
            KeyCode::Tab if self.loaded.is_some() => {
                self.view = View::Edit;
                self.param_cursor = 0;
            }
            // Toggle whether the AU supplies its own cab or feeds the built-in cab/IR.
            KeyCode::Char('c') | KeyCode::Char('C') if self.loaded.is_some() => {
                let amp_only = !params.amp_external_amp_only.load(Relaxed);
                params.amp_external_amp_only.store(amp_only, Relaxed);
                self.message = Some(if amp_only {
                    "Cab: built-in / IR".to_owned()
                } else {
                    "Cab: plugin's own".to_owned()
                });
            }
            KeyCode::Esc | KeyCode::Char('u') | KeyCode::Char('U') => self.open = false,
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
            KeyCode::Esc | KeyCode::Char('u') | KeyCode::Char('U') => self.open = false,
            _ => {}
        }
    }

    /// Adjust the selected parameter by one step in `dir` (±1). Continuous params move
    /// 1/20 of their range; stepped/indexed params move one integer step.
    fn nudge_param(&mut self, dir: i32) {
        let Some(loaded) = self.loaded.as_mut() else {
            return;
        };
        let Some(param) = loaded.params().get(self.param_cursor) else {
            return;
        };
        let step = if param.is_stepped() {
            1.0
        } else {
            (param.max - param.min) / 20.0
        };
        let target = param.value + f64::from(dir) * step;
        loaded.set_param(self.param_cursor, target);
    }

    fn activate_selection(&mut self, engine: &mut AudioEngine, params: &Params) {
        if self.cursor == 0 {
            self.message = match engine.set_external_amp(None) {
                Ok(()) => {
                    self.loaded = None;
                    params.amp_external_loaded.store(false, Relaxed);
                    params.amp_external_active.store(false, Relaxed);
                    params.amp_external_amp_only.store(false, Relaxed);
                    params.amp_external_latency.store(0, Relaxed);
                    Some("Amp override cleared".to_owned())
                }
                Err(e) => Some(format!("Clear failed: {e}")),
            };
            self.open = false;
            return;
        }

        let Some(plugin) = self.plugins.get(self.cursor - 1) else {
            return;
        };

        self.message = match au::load(plugin, self.sample_rate, self.max_block) {
            Ok((loaded, insert)) => match engine.set_external_amp(Some(insert)) {
                Ok(()) => {
                    let name = loaded.name.clone();
                    let has_params = !loaded.params().is_empty();
                    // Loading makes the AU the active amp (built-in amp+cab bypassed by
                    // default). Publish its latency so the built-in path can align, and
                    // reset the amp-only routing to the default (AU brings its own cab).
                    params.amp_external_loaded.store(true, Relaxed);
                    params.amp_external_active.store(true, Relaxed);
                    params.amp_external_amp_only.store(false, Relaxed);
                    params
                        .amp_external_latency
                        .store(loaded.latency_frames, Relaxed);
                    self.loaded = Some(loaded);
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

    /// Render the modal over the main UI. `amp_only` is the current cab-routing mode
    /// (true = the built-in cab/IR runs on the AU's output; false = the AU's own cab).
    pub(super) fn render(&self, f: &mut Frame, amp_only: bool) {
        let area = centered_rect(60, f.area());
        f.render_widget(Clear, area);

        let title = match self.view {
            View::Browse => " A U   A M P S ",
            View::Edit => " A M P   P A R A M S ",
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
                Span::styled("C", Style::default().fg(AMBER)),
                Span::styled(" cab  ", Style::default().fg(DIM)),
                Span::styled("Esc / U", Style::default().fg(AMBER)),
                Span::styled(" close", Style::default().fg(DIM)),
            ],
            View::Edit => vec![
                Span::styled("↑/↓", Style::default().fg(AMBER)),
                Span::styled(" select  ", Style::default().fg(DIM)),
                Span::styled("←/→", Style::default().fg(AMBER)),
                Span::styled(" adjust  ", Style::default().fg(DIM)),
                Span::styled("Tab", Style::default().fg(AMBER)),
                Span::styled(" browse  ", Style::default().fg(DIM)),
                Span::styled("Esc / U", Style::default().fg(AMBER)),
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
            (None, Some(name)) => {
                let latency_ms = self.loaded.as_ref().map_or(0.0, |l| l.latency_ms);
                let cab = if amp_only {
                    "built-in cab"
                } else {
                    "plugin cab"
                };
                Line::from(vec![
                    Span::styled("active: ", Style::default().fg(DIM)),
                    Span::styled(name.to_owned(), Style::default().fg(CHROME)),
                    Span::styled(
                        format!("   {latency_ms:.1} ms · {cab}"),
                        Style::default().fg(DIM),
                    ),
                ])
            }
            (None, None) => Line::from(Span::styled("no amp loaded", Style::default().fg(DIM))),
        };
        f.render_widget(
            Paragraph::new(status_line).alignment(Alignment::Center),
            footer[1],
        );
    }

    fn render_browse(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let mut lines: Vec<Line> = Vec::with_capacity(self.plugins.len() + 1);
        lines.push(entry("None — use built-in amp", self.cursor == 0));
        for (i, p) in self.plugins.iter().enumerate() {
            lines.push(entry(&p.name, self.cursor == i + 1));
        }
        if self.plugins.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (no Audio Units found)",
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
fn param_row(p: &AuParam, selected: bool) -> Line<'static> {
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
        Span::styled(
            format!("  {}", p.display_value()),
            Style::default().fg(AMBER),
        ),
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

fn entry(name: &str, selected: bool) -> Line<'static> {
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
    Line::from(vec![
        Span::styled(
            prefix.to_owned(),
            Style::default().fg(if selected { ORANGE } else { DIM }),
        ),
        Span::styled(name.to_owned(), name_style),
    ])
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
