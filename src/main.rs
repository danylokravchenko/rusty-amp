mod audio;
mod dsp;
mod preset;
mod ui;

use anyhow::Result;
use std::sync::Arc;

fn main() -> Result<()> {
    let params = Arc::new(dsp::Params::new());
    let levels = Arc::new(dsp::Levels::new());

    // ── Preset selection (before audio starts so values are ready) ────────────
    let presets = preset::load_all();
    if let Some(chosen) = preset::prompt_user(&presets) {
        chosen.apply(&params);
        println!("Loaded: {}\n", chosen.name);
    }

    // ── Audio engine (device selection + probe + stream start) ────────────────
    let _engine = audio::start(Arc::clone(&params), Arc::clone(&levels))?;

    // ── TUI (blocks until user quits) ─────────────────────────────────────────
    ui::run(params, levels)?;

    Ok(())
}
