use anyhow::Result;
use std::sync::Arc;

use rusty_amp::{dsp, preset, recording, ui};

fn main() -> Result<()> {
    let params = Arc::new(dsp::Params::new());
    let levels = Arc::new(dsp::Levels::new());
    let presets = preset::load_all();
    let recording = Arc::new(recording::RecordingState::new());

    // TUI starts immediately; device selection happens inside via modals.
    ui::run(params, levels, presets, recording)?;

    Ok(())
}
