use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::{AmpModel, Params};

// ── TOML schema ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct Preset {
    pub name: String,
    pub description: Option<String>,
    pub tube_screamer: TsSection,
    pub distortion: Option<DsSection>,
    pub amp: AmpSection,
    pub reverb: ReverbSection,
}

#[derive(Debug, Deserialize)]
pub struct TsSection {
    pub enabled: Option<bool>,
    pub drive: f32,
    pub tone: f32,
    pub level: f32,
}

#[derive(Debug, Deserialize)]
pub struct DsSection {
    pub enabled: Option<bool>,
    pub drive: f32,
    pub tone: f32,
    pub level: f32,
}

#[derive(Debug, Deserialize)]
pub struct AmpSection {
    /// "marshall" (default) or "mesa"
    pub model: Option<String>,
    pub gain: f32,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub master: f32,
}

#[derive(Debug, Deserialize)]
pub struct ReverbSection {
    pub enabled: Option<bool>,
    pub room: f32,
    pub damp: f32,
    pub mix: f32,
}

impl Preset {
    pub fn load(path: &Path) -> Result<Self> {
        let src =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&src).with_context(|| format!("parsing {}", path.display()))
    }

    /// Write all preset values into the shared atomic params.
    pub fn apply(&self, params: &Params) {
        let ts = &self.tube_screamer;
        params.ts_enabled.store(ts.enabled.unwrap_or(true), Relaxed);
        params.ts_drive.store(ts.drive.clamp(0.0, 1.0), Relaxed);
        params.ts_tone.store(ts.tone.clamp(0.0, 1.0), Relaxed);
        params.ts_level.store(ts.level.clamp(0.0, 1.0), Relaxed);

        if let Some(ds) = &self.distortion {
            params.ds_enabled.store(ds.enabled.unwrap_or(true), Relaxed);
            params.ds_drive.store(ds.drive.clamp(0.0, 1.0), Relaxed);
            params.ds_tone.store(ds.tone.clamp(0.0, 1.0), Relaxed);
            params.ds_level.store(ds.level.clamp(0.0, 1.0), Relaxed);
        } else {
            params.ds_enabled.store(false, Relaxed);
        }

        let amp = &self.amp;
        let model = match amp.model.as_deref() {
            Some("mesa") => AmpModel::Mesa,
            Some("randall") => AmpModel::Randall,
            _ => AmpModel::Marshall,
        };
        params.amp_model.store(model as u8, Relaxed);
        params.amp_gain.store(amp.gain.clamp(0.0, 1.0), Relaxed);
        params.amp_bass.store(amp.bass.clamp(0.0, 1.0), Relaxed);
        params.amp_mid.store(amp.mid.clamp(0.0, 1.0), Relaxed);
        params.amp_treble.store(amp.treble.clamp(0.0, 1.0), Relaxed);
        params.amp_master.store(amp.master.clamp(0.0, 1.0), Relaxed);

        let rev = &self.reverb;
        params
            .rev_enabled
            .store(rev.enabled.unwrap_or(true), Relaxed);
        params.rev_room.store(rev.room.clamp(0.0, 1.0), Relaxed);
        params.rev_damp.store(rev.damp.clamp(0.0, 1.0), Relaxed);
        params.rev_mix.store(rev.mix.clamp(0.0, 1.0), Relaxed);
    }
}

// ── Discovery ─────────────────────────────────────────────────────────────────

pub fn find_preset_files() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = vec![PathBuf::from("presets")];

    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".config").join("rusty-amp").join("presets"));
    }

    let mut files: Vec<PathBuf> = Vec::new();
    for dir in &dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut dir_files: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
                .collect();
            dir_files.sort();
            files.extend(dir_files);
        }
    }
    files
}

pub fn load_all() -> Vec<Preset> {
    find_preset_files()
        .into_iter()
        .filter_map(|path| {
            Preset::load(&path)
                .map_err(|e| eprintln!("Warning: skipping preset {}: {e}", path.display()))
                .ok()
        })
        .collect()
}
