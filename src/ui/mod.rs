mod config;
mod draw;
mod input;
mod presets;
mod styles;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use crate::dsp::{Levels, Params};
use crate::preset::Preset;

use draw::draw;
use input::{cycle_amp, next_section, nudge, prev_section, toggle_pedal};
use presets::render_preset_modal;

pub fn run(params: Arc<Params>, levels: Arc<Levels>, presets: Vec<Preset>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut focus: Option<usize> = None;
    let mut preset_open = false;
    let mut preset_cursor = 0usize; // 0 = Default, 1..=N = preset index

    loop {
        terminal.draw(|f| {
            draw(f, &params, &levels, focus);
            if preset_open {
                render_preset_modal(f, &presets, preset_cursor);
            }
        })?;

        if event::poll(Duration::from_millis(30))?
            && let Event::Key(key) = event::read()?
        {
            if preset_open {
                let total = presets.len() + 1; // +1 for "Default"
                match key.code {
                    KeyCode::Up => {
                        preset_cursor = preset_cursor.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        preset_cursor = (preset_cursor + 1).min(total - 1);
                    }
                    KeyCode::Enter => {
                        if preset_cursor > 0 {
                            presets[preset_cursor - 1].apply(&params);
                        }
                        preset_open = false;
                    }
                    KeyCode::Esc | KeyCode::Char('p') | KeyCode::Char('P') => {
                        preset_open = false;
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        break;
                    }
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        preset_open = true;
                        preset_cursor = 0;
                    }
                    KeyCode::Tab => focus = next_section(focus),
                    KeyCode::BackTab => focus = prev_section(focus),
                    KeyCode::Right => {
                        focus = match focus {
                            None => Some(0),
                            Some(i) => Some((i + 1) % config::KNOBS.len()),
                        };
                    }
                    KeyCode::Left => {
                        focus = match focus {
                            None => Some(config::KNOBS.len() - 1),
                            Some(0) => None,
                            Some(i) => Some(i - 1),
                        };
                    }
                    KeyCode::Up | KeyCode::Char('+') | KeyCode::Char('=') => match focus {
                        None => cycle_amp(&params, 1),
                        Some(i) => nudge(&params, i, 0.05),
                    },
                    KeyCode::Down | KeyCode::Char('-') => match focus {
                        None => cycle_amp(&params, -1),
                        Some(i) => nudge(&params, i, -0.05),
                    },
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
