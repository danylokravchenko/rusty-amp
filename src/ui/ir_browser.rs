//! Impulse-response browser modal: scan, load, clear and A/B a user-supplied `.wav`
//! cabinet IR against the built-in cabs.
//!
//! Mirrors the CLAP [`super::plugins`] browser, but for cabinet IRs. Decoding +
//! resampling happen off the audio thread in [`crate::dsp::cab::load_ir`]; the
//! finished cab is handed to the realtime callback lock-free via
//! [`AudioEngine::set_external_cab`].

use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use super::styles::{AMBER, CHROME, DIM, ORANGE, SAFE, WARN};
use crate::audio::AudioEngine;
use crate::dsp::Params;
use crate::dsp::cab::{ExternalIrCab, MAX_IR_LEN, load_ir};

/// Shown when the user tries to load/toggle an IR while an external amp (AU) is the
/// active amp — it replaces the built-in amp+cab, so an IR is inert until they switch
/// back to the built-in amp (`Z`).
const AMP_ACTIVE_WARNING: &str =
    "External amp active — IR has no effect (press Z for built-in amp)";

/// One discovered IR file: full path plus a short label (its stem, with a parent
/// hint so same-named files in different folders are distinguishable).
struct IrFile {
    path: PathBuf,
    label: String,
}

/// All IR-browser state, kept on the UI thread.
pub(super) struct IrBrowser {
    pub(super) open: bool,
    cursor: usize,
    files: Vec<IrFile>,
    /// Name of the currently loaded IR (file stem), if any.
    loaded: Option<String>,
    message: Option<String>,
    sample_rate: f32,
}

impl IrBrowser {
    pub(super) fn new(sample_rate: f32) -> Self {
        Self {
            open: false,
            cursor: 0,
            files: Vec::new(),
            loaded: None,
            message: None,
            sample_rate,
        }
    }

    /// Open the modal, (re)scanning the standard IR locations.
    pub(super) fn open(&mut self) {
        self.files = scan();
        self.cursor = 0;
        self.open = true;
    }

    /// Name of the loaded IR, for the main header.
    pub(super) fn loaded_name(&self) -> Option<&str> {
        self.loaded.as_deref()
    }

    /// Handle a keypress while the modal is open.
    pub(super) fn handle_key(&mut self, code: KeyCode, engine: &mut AudioEngine, params: &Params) {
        // Entry 0 is the "built-in cabs" row; entries 1.. map to `self.files`.
        let total = self.files.len() + 1;
        match code {
            KeyCode::Up => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Down => self.cursor = (self.cursor + 1).min(total - 1),
            KeyCode::Enter => self.activate_selection(engine, params),
            // Toggle the loaded IR active/inactive without leaving the modal.
            KeyCode::Char('x') | KeyCode::Char('X') if self.loaded.is_some() => {
                if params.amp_external_active.load(Relaxed) {
                    self.message = Some(AMP_ACTIVE_WARNING.to_owned());
                } else {
                    let now = !params.cab_external_active.load(Relaxed);
                    params.cab_external_active.store(now, Relaxed);
                    self.message = Some(if now {
                        "External IR active".to_owned()
                    } else {
                        "Built-in cab active".to_owned()
                    });
                }
            }
            KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('I') => self.open = false,
            _ => {}
        }
    }

    fn activate_selection(&mut self, engine: &mut AudioEngine, params: &Params) {
        // While an external amp (AU) is active it replaces the built-in amp *and* cab,
        // so an IR would have no effect. Block loading and say so rather than silently
        // doing nothing. Clearing (cursor 0) is still allowed.
        if self.cursor != 0 && params.amp_external_active.load(Relaxed) {
            self.message = Some(AMP_ACTIVE_WARNING.to_owned());
            return;
        }
        if self.cursor == 0 {
            // Clear the external IR and fall back to the built-in cabs.
            self.message = match engine.set_external_cab(None) {
                Ok(()) => {
                    params.cab_external_active.store(false, Relaxed);
                    params.cab_external_loaded.store(false, Relaxed);
                    self.loaded = None;
                    Some("Built-in cab active".to_owned())
                }
                Err(e) => Some(format!("Clear failed: {e}")),
            };
            self.open = false;
            return;
        }

        let Some(file) = self.files.get(self.cursor - 1) else {
            return;
        };

        self.message = match load_ir(&file.path, self.sample_rate, MAX_IR_LEN) {
            Ok(loaded) => {
                let name = loaded.name.clone();
                let cab = Box::new(ExternalIrCab::new(self.sample_rate, loaded));
                match engine.set_external_cab(Some(cab)) {
                    Ok(()) => {
                        params.cab_external_loaded.store(true, Relaxed);
                        params.cab_external_active.store(true, Relaxed);
                        self.loaded = Some(name.clone());
                        self.open = false;
                        Some(format!("Loaded {name}"))
                    }
                    Err(e) => Some(format!("Load failed: {e}")),
                }
            }
            Err(e) => Some(format!("Load failed: {e}")),
        };
    }

    /// Render the modal over the main UI. `amp_ext_active` flags that a hosted AU amp
    /// is the active amp, in which case IRs are inert and the modal says so.
    pub(super) fn render(&self, f: &mut Frame, amp_ext_active: bool) {
        let area = centered_rect(60, f.area());
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(ORANGE))
            .title(Span::styled(
                " C A B I N E T   I R s ",
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(area);
        f.render_widget(block, area);

        // A one-line warning banner sits above the list whenever an external amp is
        // active, so the "no effect" state is obvious before the user even tries.
        let banner_h = u16::from(amp_ext_active);
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(banner_h),
                Constraint::Min(1),
                Constraint::Length(2),
            ])
            .split(inner);

        if amp_ext_active {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!("⚠ {AMP_ACTIVE_WARNING}"),
                    Style::default().fg(WARN).add_modifier(Modifier::BOLD),
                )))
                .alignment(Alignment::Center),
                rows[0],
            );
        }

        self.render_list(f, rows[1]);

        let footer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(rows[2]);

        let hint = vec![
            Span::styled("↑/↓", Style::default().fg(AMBER)),
            Span::styled(" navigate  ", Style::default().fg(DIM)),
            Span::styled("Enter", Style::default().fg(AMBER)),
            Span::styled(" load/clear  ", Style::default().fg(DIM)),
            Span::styled("X", Style::default().fg(AMBER)),
            Span::styled(" A/B  ", Style::default().fg(DIM)),
            Span::styled("Esc / I", Style::default().fg(AMBER)),
            Span::styled(" close", Style::default().fg(DIM)),
        ];
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
                Span::styled("loaded: ", Style::default().fg(DIM)),
                Span::styled(name.to_owned(), Style::default().fg(CHROME)),
            ]),
            (None, None) => Line::from(Span::styled(
                "no IR loaded — using built-in cabs",
                Style::default().fg(DIM),
            )),
        };
        f.render_widget(
            Paragraph::new(status_line).alignment(Alignment::Center),
            footer[1],
        );
    }

    fn render_list(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let mut lines: Vec<Line> = Vec::with_capacity(self.files.len() + 1);
        lines.push(entry("Built-in cabs (no IR)", "", self.cursor == 0));
        for (i, file) in self.files.iter().enumerate() {
            let detail = file
                .path
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or_default();
            lines.push(entry(&file.label, detail, self.cursor == i + 1));
        }
        if self.files.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (no .wav IRs found — drop files in ./irs or ~/.config/rusty-amp/irs)",
                Style::default().fg(DIM),
            )));
        }

        let visible = area.height as usize;
        let offset = self.cursor.saturating_sub(visible.saturating_sub(1));
        let visible_lines: Vec<Line> = lines.into_iter().skip(offset).collect();
        f.render_widget(Paragraph::new(visible_lines), area);
    }
}

/// Standard IR locations, scanned (recursively, bounded depth) for `.wav` files:
/// `$RUSTY_AMP_IR_DIR`, `./irs` next to the binary's working dir, and
/// `~/.config/rusty-amp/irs`.
fn scan() -> Vec<IrFile> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = std::env::var("RUSTY_AMP_IR_DIR") {
        roots.push(PathBuf::from(dir));
    }
    roots.push(PathBuf::from("irs"));
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".config").join("rusty-amp").join("irs"));
    }

    let mut out: Vec<IrFile> = Vec::new();
    for root in roots {
        collect_wavs(&root, 3, &mut out);
    }
    // Stable, de-duplicated ordering by label then path.
    out.sort_by(|a, b| a.label.cmp(&b.label).then(a.path.cmp(&b.path)));
    out.dedup_by(|a, b| a.path == b.path);
    out
}

/// Recursively gather `.wav` files under `dir` up to `depth` levels deep.
fn collect_wavs(dir: &PathBuf, depth: usize, out: &mut Vec<IrFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth > 0 {
                collect_wavs(&path, depth - 1, out);
            }
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
        {
            let label = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("IR")
                .to_owned();
            out.push(IrFile { path, label });
        }
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
