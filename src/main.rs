mod audio;
mod dsp;
mod preset;
mod ui;

use anyhow::Result;
use std::sync::Arc;

fn main() -> Result<()> {
    let params = Arc::new(dsp::Params::new());
    let levels = Arc::new(dsp::Levels::new());

    // Presets are loaded once at startup and passed into the TUI,
    // where the user can browse and apply them at any time with P.
    let presets = preset::load_all();

    // ── Audio engine (device selection + stream start) ────────────────────────
    let _engine = audio::start(Arc::clone(&params), Arc::clone(&levels))?;

    // ── TUI (blocks until user quits) ─────────────────────────────────────────
    ui::run(params, levels, presets)?;

    Ok(())
}
