mod config;
mod draw;
mod input;
#[cfg(feature = "clap")]
mod plugins;
mod presets;
mod setup;
mod styles;
mod tuner;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use crate::dsp::{Levels, Params, Tuner};
use crate::preset::Preset;
use crate::recording::RecordingState;

use config::{ADD_TILE, PEDALS, pedal_of};
use draw::{draw, render_add_pedal_modal};
use input::{
    add_pedal, cycle_amp, cycle_cab, nav_knob, next_section, nudge, prev_section, remove_pedal,
    toggle_pedal,
};
use presets::{render_preset_modal, render_save_dialog};

/// Board membership derived from the live enabled flags (one entry per pedal).
fn sync_board(params: &Params) -> Vec<bool> {
    PEDALS
        .iter()
        .map(|p| (p.enabled)(params).load(std::sync::atomic::Ordering::Relaxed))
        .collect()
}

pub fn run(
    params: Arc<Params>,
    levels: Arc<Levels>,
    tuner: Arc<Tuner>,
    presets: Vec<Preset>,
    recording: Arc<RecordingState>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // ── Device selection (TUI modals before audio starts) ─────────────────────
    let devices = crate::audio::list_devices()?;
    let selection = setup::run(&mut terminal, &devices, &params, &levels);

    // Tear down on quit during setup
    let selection = match selection {
        Ok(s) => s,
        Err(_) => {
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            return Ok(());
        }
    };

    // ── Start audio engine ────────────────────────────────────────────────────
    #[cfg_attr(not(feature = "clap"), allow(unused_mut, unused_variables))]
    let mut engine = crate::audio::start(
        selection.input_idx,
        selection.guitar_ch,
        selection.output_idx,
        Arc::clone(&params),
        Arc::clone(&levels),
        Arc::clone(&recording),
        Arc::clone(&tuner),
    )?;

    // ── Plugin browser (CLAP insert) ──────────────────────────────────────────
    #[cfg(feature = "clap")]
    let mut browser =
        plugins::PluginBrowser::new(engine.sample_rate(), crate::audio::MAX_BLOCK as u32);

    // ── Main UI loop ──────────────────────────────────────────────────────────
    let mut focus: Option<usize> = None;
    // Board membership: a pedal is on the board iff it is enabled. Off-board
    // pedals are bypassed in the DSP and hidden from the rig. Rebuilt with
    // `sync_board` whenever a preset rewrites the enabled flags.
    let mut board: Vec<bool> = sync_board(&params);
    let mut add_open = false;
    let mut add_cursor = 0usize;
    let mut preset_open = false;
    let mut preset_cursor = 0usize;
    let mut presets = presets;
    let mut save_open = false;
    let mut save_name = String::new();
    let mut save_desc = String::new();
    let mut save_field = 0usize; // 0 = name, 1 = description
    let mut save_error: Option<String> = None;
    let mut tick: u64 = 0;
    let mut save_msg: Option<(String, std::time::Instant)> = None;
    let mut tuner_open = false;

    loop {
        tick = tick.wrapping_add(1);
        let blink = (tick / 15).is_multiple_of(2);
        let rec_active = recording.active.load(std::sync::atomic::Ordering::Relaxed);

        // Clear save message after 4 seconds
        if let Some((_, ts)) = &save_msg
            && ts.elapsed().as_secs() >= 4
        {
            save_msg = None;
        }

        let status = save_msg.as_ref().map(|(msg, _)| msg.as_str());
        // The loaded plugin (if any) is shown in the header, not the status line, so
        // the help/status footer stays intact while a plugin is active.
        #[cfg(feature = "clap")]
        let plugin_name = browser.loaded_name();
        #[cfg(not(feature = "clap"))]
        let plugin_name: Option<&str> = None;

        terminal.draw(|f| {
            draw(
                f,
                &params,
                &levels,
                focus,
                &board,
                rec_active,
                blink,
                status,
                plugin_name,
            );
            if add_open {
                let available: Vec<usize> = (0..PEDALS.len()).filter(|&i| !board[i]).collect();
                render_add_pedal_modal(f, &available, add_cursor);
            }
            if preset_open {
                render_preset_modal(f, &presets, preset_cursor);
            }
            if save_open {
                render_save_dialog(f, &save_name, &save_desc, save_field, save_error.as_deref());
            }
            #[cfg(feature = "clap")]
            if browser.open {
                browser.render(f);
            }
            if tuner_open {
                tuner::render_tuner(f, &tuner);
            }
        })?;

        if event::poll(Duration::from_millis(30))?
            && let Event::Key(key) = event::read()?
        {
            #[cfg(feature = "clap")]
            if browser.open {
                browser.handle_key(key.code, &mut engine);
                continue;
            }

            if tuner_open {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('t') | KeyCode::Char('T') | KeyCode::Char('q') => {
                        tuner_open = false;
                        tuner
                            .active
                            .store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                    _ => {}
                }
            } else if save_open {
                match key.code {
                    KeyCode::Esc => {
                        save_open = false;
                        save_error = None;
                    }
                    KeyCode::Tab => {
                        save_field = 1 - save_field;
                    }
                    KeyCode::Enter => {
                        if save_name.trim().is_empty() {
                            save_error = Some("Name cannot be empty".to_string());
                        } else {
                            let preset = crate::preset::Preset::from_params(
                                save_name.trim().to_string(),
                                if save_desc.trim().is_empty() {
                                    None
                                } else {
                                    Some(save_desc.trim().to_string())
                                },
                                &params,
                            );
                            match preset.save_to_user_dir() {
                                Ok(_) => {
                                    presets = crate::preset::load_all();
                                    save_open = false;
                                    save_name.clear();
                                    save_desc.clear();
                                    save_error = None;
                                }
                                Err(e) => {
                                    save_error = Some(format!("Save failed: {e}"));
                                }
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        if save_field == 0 {
                            save_name.pop();
                        } else {
                            save_desc.pop();
                        }
                        save_error = None;
                    }
                    KeyCode::Char(c) => {
                        if save_field == 0 {
                            save_name.push(c);
                        } else {
                            save_desc.push(c);
                        }
                        save_error = None;
                    }
                    _ => {}
                }
            } else if preset_open {
                let total = presets.len() + 1;
                match key.code {
                    KeyCode::Up => {
                        preset_cursor = preset_cursor.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        preset_cursor = (preset_cursor + 1).min(total - 1);
                    }
                    KeyCode::Enter => {
                        if preset_cursor == 0 {
                            params.reset_to_defaults();
                        } else {
                            presets[preset_cursor - 1].apply(&params);
                        }
                        // The preset rewrote the enabled flags, so rebuild the
                        // board and drop focus if it landed on a removed pedal.
                        board = sync_board(&params);
                        if let Some(i) = focus
                            && let Some(pi) = pedal_of(i)
                            && !board[pi]
                        {
                            focus = None;
                        }
                        preset_open = false;
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        preset_open = false;
                        save_open = true;
                        save_name.clear();
                        save_desc.clear();
                        save_field = 0;
                        save_error = None;
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') if preset_cursor > 0 => {
                        let p = &presets[preset_cursor - 1];
                        if p.source == crate::preset::PresetSource::User {
                            let _ = p.delete();
                            presets = crate::preset::load_all();
                            preset_cursor = preset_cursor.saturating_sub(1);
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('p') | KeyCode::Char('P') => {
                        preset_open = false;
                    }
                    _ => {}
                }
            } else if add_open {
                let available: Vec<usize> = (0..PEDALS.len()).filter(|&i| !board[i]).collect();
                match key.code {
                    KeyCode::Up => add_cursor = add_cursor.saturating_sub(1),
                    KeyCode::Down if !available.is_empty() => {
                        add_cursor = (add_cursor + 1).min(available.len() - 1);
                    }
                    KeyCode::Enter => {
                        if let Some(&pi) = available.get(add_cursor) {
                            add_pedal(&params, &mut board, pi);
                            focus = Some(PEDALS[pi].start);
                        }
                        add_open = false;
                    }
                    KeyCode::Esc => add_open = false,
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        break;
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        if rec_active {
                            match recording.stop_and_save() {
                                Ok(path) => {
                                    let msg = format!("Saved: {}", path.display());
                                    save_msg = Some((msg, std::time::Instant::now()));
                                }
                                Err(e) => {
                                    save_msg = Some((
                                        format!("Save failed: {e}"),
                                        std::time::Instant::now(),
                                    ));
                                }
                            }
                        } else {
                            recording.start();
                        }
                    }
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        preset_open = true;
                        preset_cursor = 0;
                    }
                    KeyCode::Char('t') | KeyCode::Char('T') => {
                        tuner_open = true;
                        tuner
                            .active
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    #[cfg(feature = "clap")]
                    KeyCode::Char('v') | KeyCode::Char('V') => browser.open(),
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        save_open = true;
                        save_name.clear();
                        save_desc.clear();
                        save_field = 0;
                        save_error = None;
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        cycle_amp(&params, 1);
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        cycle_cab(&params);
                    }
                    KeyCode::Tab => focus = next_section(focus, &board),
                    KeyCode::BackTab => focus = prev_section(focus, &board),
                    KeyCode::Right => focus = nav_knob(focus, &board, 1),
                    KeyCode::Left => focus = nav_knob(focus, &board, -1),
                    KeyCode::Up | KeyCode::Char('+') | KeyCode::Char('=') => match focus {
                        None => cycle_amp(&params, 1),
                        Some(ADD_TILE) => {}
                        Some(i) => nudge(&params, i, 0.05),
                    },
                    KeyCode::Down | KeyCode::Char('-') => match focus {
                        None => cycle_amp(&params, -1),
                        Some(ADD_TILE) => {}
                        Some(i) => nudge(&params, i, -0.05),
                    },
                    KeyCode::Enter if focus == Some(ADD_TILE) => {
                        add_open = true;
                        add_cursor = 0;
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        if let Some(i) = focus
                            && let Some(pi) = pedal_of(i)
                        {
                            remove_pedal(&params, &mut board, pi);
                            focus = Some(ADD_TILE);
                        }
                    }
                    KeyCode::Char(' ') => match focus {
                        Some(ADD_TILE) => {
                            add_open = true;
                            add_cursor = 0;
                        }
                        Some(i) => toggle_pedal(&params, i),
                        None => {}
                    },
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
