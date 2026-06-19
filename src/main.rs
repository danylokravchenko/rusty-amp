mod audio;
mod dsp;
mod preset;
mod ui;

use anyhow::Result;
use std::sync::Arc;

fn main() -> Result<()> {
    let params = Arc::new(dsp::Params::new());
    let levels = Arc::new(dsp::Levels::new());
    let presets = preset::load_all();

    // TUI starts immediately; device selection happens inside via modals.
    ui::run(params, levels, presets)?;

    Ok(())
}
