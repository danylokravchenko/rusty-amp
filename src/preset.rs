use anyhow::{Context, Result};
use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::{AmpModel, CabModel, Params};

#[derive(Embed)]
#[folder = "presets/"]
#[include = "*.toml"]
struct BundledPresets;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresetSource {
    System,
    #[default]
    User,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Preset {
    pub name: String,
    pub description: Option<String>,
    #[serde(skip)]
    pub source: PresetSource,
    #[serde(skip)]
    pub path: Option<PathBuf>,
    pub noise_gate: Option<NgSection>,
    pub compressor: Option<CmpSection>,
    pub fuzz: Option<FuzzSection>,
    pub tube_screamer: TsSection,
    pub distortion: Option<DsSection>,
    pub preamp_eq: Option<PeqSection>,
    pub amp: AmpSection,
    pub cabinet: Option<CabSection>,
    pub eq: Option<EqSection>,
    pub flanger: Option<FlangerSection>,
    pub delay: Option<DelaySection>,
    pub reverb: ReverbSection,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NgSection {
    pub enabled: Option<bool>,
    pub threshold: f32,
    pub release: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CmpSection {
    pub enabled: Option<bool>,
    pub sustain: f32,
    pub attack: f32,
    pub level: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PeqSection {
    pub enabled: Option<bool>,
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FuzzSection {
    pub enabled: Option<bool>,
    pub fuzz: f32,
    pub tone: f32,
    pub level: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TsSection {
    pub enabled: Option<bool>,
    pub drive: f32,
    pub tone: f32,
    pub level: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DsSection {
    pub enabled: Option<bool>,
    pub drive: f32,
    pub tone: f32,
    pub level: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AmpSection {
    /// "marshall" | "mesa" | "randall"
    pub model: Option<String>,
    pub gain: f32,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    #[serde(default = "presence_default")]
    pub presence: f32,
    pub master: f32,
}

fn presence_default() -> f32 {
    0.5
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CabSection {
    /// "mesa" (default) | "marshall"
    pub model: Option<String>,
    /// 0.0 = edge (off-axis, dark) … 1.0 = center (on-axis, bright). Default 0.5.
    #[serde(default = "mic_pos_default")]
    pub mic_pos: f32,
    /// 0.0 = close SM57 dynamic … 1.0 = R121 ribbon. Default 0.15.
    #[serde(default = "mic_blend_default")]
    pub mic_blend: f32,
    /// 0.0 = dry close mic only … 1.0 = full ambient room mic. Default 0.15.
    #[serde(default = "mic_room_default")]
    pub mic_room: f32,
}

fn mic_pos_default() -> f32 {
    0.5
}

fn mic_blend_default() -> f32 {
    0.15
}

fn mic_room_default() -> f32 {
    0.15
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EqSection {
    pub enabled: Option<bool>,
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DelaySection {
    pub enabled: Option<bool>,
    pub time: f32,
    pub feedback: f32,
    pub mix: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FlangerSection {
    pub enabled: Option<bool>,
    pub rate: f32,
    pub depth: f32,
    pub feedback: f32,
    pub mix: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReverbSection {
    pub enabled: Option<bool>,
    pub room: f32,
    pub damp: f32,
    pub mix: f32,
}

impl Preset {
    pub fn load(path: &Path, source: PresetSource) -> Result<Self> {
        let src =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut preset: Self =
            toml::from_str(&src).with_context(|| format!("parsing {}", path.display()))?;
        preset.source = source;
        preset.path = Some(path.to_path_buf());
        Ok(preset)
    }

    pub fn delete(&self) -> Result<()> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("preset has no file path"))?;
        std::fs::remove_file(path).with_context(|| format!("deleting {}", path.display()))
    }

    pub fn from_params(name: String, description: Option<String>, params: &Params) -> Self {
        let amp_model = AmpModel::from_u8(params.amp_model.load(Relaxed));
        let amp_model_str = match amp_model {
            AmpModel::Marshall => "marshall",
            AmpModel::Mesa => "mesa",
            AmpModel::Randall => "randall",
        };
        let cab_model = CabModel::from_u8(params.cab_model.load(Relaxed));
        let cab_model_str = match cab_model {
            CabModel::Mesa => "mesa",
            CabModel::Marshall => "marshall",
            CabModel::Orange => "orange",
        };
        Self {
            name,
            description,
            source: PresetSource::User,
            path: None,
            noise_gate: Some(NgSection {
                enabled: Some(params.ng_enabled.load(Relaxed)),
                threshold: params.ng_threshold.load(Relaxed),
                release: params.ng_release.load(Relaxed),
            }),
            compressor: Some(CmpSection {
                enabled: Some(params.cmp_enabled.load(Relaxed)),
                sustain: params.cmp_sustain.load(Relaxed),
                attack: params.cmp_attack.load(Relaxed),
                level: params.cmp_level.load(Relaxed),
            }),
            fuzz: Some(FuzzSection {
                enabled: Some(params.fz_enabled.load(Relaxed)),
                fuzz: params.fz_fuzz.load(Relaxed),
                tone: params.fz_tone.load(Relaxed),
                level: params.fz_level.load(Relaxed),
            }),
            tube_screamer: TsSection {
                enabled: Some(params.ts_enabled.load(Relaxed)),
                drive: params.ts_drive.load(Relaxed),
                tone: params.ts_tone.load(Relaxed),
                level: params.ts_level.load(Relaxed),
            },
            distortion: Some(DsSection {
                enabled: Some(params.ds_enabled.load(Relaxed)),
                drive: params.ds_drive.load(Relaxed),
                tone: params.ds_tone.load(Relaxed),
                level: params.ds_level.load(Relaxed),
            }),
            preamp_eq: Some(PeqSection {
                enabled: Some(params.peq_enabled.load(Relaxed)),
                low: params.peq_low.load(Relaxed),
                mid: params.peq_mid.load(Relaxed),
                high: params.peq_high.load(Relaxed),
            }),
            amp: AmpSection {
                model: Some(amp_model_str.to_string()),
                gain: params.amp_gain.load(Relaxed),
                bass: params.amp_bass.load(Relaxed),
                mid: params.amp_mid.load(Relaxed),
                treble: params.amp_treble.load(Relaxed),
                presence: params.amp_presence.load(Relaxed),
                master: params.amp_master.load(Relaxed),
            },
            cabinet: Some(CabSection {
                model: Some(cab_model_str.to_string()),
                mic_pos: params.mic_pos.load(Relaxed),
                mic_blend: params.mic_blend.load(Relaxed),
                mic_room: params.mic_room.load(Relaxed),
            }),
            eq: Some(EqSection {
                enabled: Some(params.eq_enabled.load(Relaxed)),
                low: params.eq_low.load(Relaxed),
                mid: params.eq_mid.load(Relaxed),
                high: params.eq_high.load(Relaxed),
            }),
            delay: Some(DelaySection {
                enabled: Some(params.delay_enabled.load(Relaxed)),
                time: params.delay_time.load(Relaxed),
                feedback: params.delay_feedback.load(Relaxed),
                mix: params.delay_mix.load(Relaxed),
            }),
            flanger: Some(FlangerSection {
                enabled: Some(params.fl_enabled.load(Relaxed)),
                rate: params.fl_rate.load(Relaxed),
                depth: params.fl_depth.load(Relaxed),
                feedback: params.fl_feedback.load(Relaxed),
                mix: params.fl_mix.load(Relaxed),
            }),
            reverb: ReverbSection {
                enabled: Some(params.rev_enabled.load(Relaxed)),
                room: params.rev_room.load(Relaxed),
                damp: params.rev_damp.load(Relaxed),
                mix: params.rev_mix.load(Relaxed),
            },
        }
    }

    pub fn save_to_user_dir(&self) -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot find home dir"))?;
        let dir = home.join(".config").join("rusty-amp").join("presets");
        std::fs::create_dir_all(&dir)?;

        let filename = self
            .name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();
        let path = dir.join(format!("{filename}.toml"));

        let toml_str = toml::to_string_pretty(self).with_context(|| "serializing preset")?;
        std::fs::write(&path, toml_str)?;
        Ok(path)
    }

    /// Write all preset values into the shared atomic params.
    pub fn apply(&self, params: &Params) {
        if let Some(ng) = &self.noise_gate {
            params.ng_enabled.store(ng.enabled.unwrap_or(true), Relaxed);
            params
                .ng_threshold
                .store(ng.threshold.clamp(0.0, 1.0), Relaxed);
            params.ng_release.store(ng.release.clamp(0.0, 1.0), Relaxed);
        }

        if let Some(cmp) = &self.compressor {
            params
                .cmp_enabled
                .store(cmp.enabled.unwrap_or(true), Relaxed);
            params
                .cmp_sustain
                .store(cmp.sustain.clamp(0.0, 1.0), Relaxed);
            params.cmp_attack.store(cmp.attack.clamp(0.0, 1.0), Relaxed);
            params.cmp_level.store(cmp.level.clamp(0.0, 1.0), Relaxed);
        } else {
            params.cmp_enabled.store(false, Relaxed);
        }

        if let Some(fz) = &self.fuzz {
            params.fz_enabled.store(fz.enabled.unwrap_or(true), Relaxed);
            params.fz_fuzz.store(fz.fuzz.clamp(0.0, 1.0), Relaxed);
            params.fz_tone.store(fz.tone.clamp(0.0, 1.0), Relaxed);
            params.fz_level.store(fz.level.clamp(0.0, 1.0), Relaxed);
        } else {
            params.fz_enabled.store(false, Relaxed);
        }

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

        if let Some(peq) = &self.preamp_eq {
            params
                .peq_enabled
                .store(peq.enabled.unwrap_or(true), Relaxed);
            params.peq_low.store(peq.low.clamp(0.0, 1.0), Relaxed);
            params.peq_mid.store(peq.mid.clamp(0.0, 1.0), Relaxed);
            params.peq_high.store(peq.high.clamp(0.0, 1.0), Relaxed);
        } else {
            params.peq_enabled.store(false, Relaxed);
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
        params
            .amp_presence
            .store(amp.presence.clamp(0.0, 1.0), Relaxed);
        params.amp_master.store(amp.master.clamp(0.0, 1.0), Relaxed);

        if let Some(cab) = &self.cabinet {
            let cab_model = match cab.model.as_deref() {
                Some("marshall") => CabModel::Marshall,
                Some("orange") => CabModel::Orange,
                _ => CabModel::Mesa,
            };
            params.cab_model.store(cab_model as u8, Relaxed);
            params.mic_pos.store(cab.mic_pos.clamp(0.0, 1.0), Relaxed);
            params
                .mic_blend
                .store(cab.mic_blend.clamp(0.0, 1.0), Relaxed);
            params.mic_room.store(cab.mic_room.clamp(0.0, 1.0), Relaxed);
        }

        if let Some(eq) = &self.eq {
            params.eq_enabled.store(eq.enabled.unwrap_or(true), Relaxed);
            params.eq_low.store(eq.low.clamp(0.0, 1.0), Relaxed);
            params.eq_mid.store(eq.mid.clamp(0.0, 1.0), Relaxed);
            params.eq_high.store(eq.high.clamp(0.0, 1.0), Relaxed);
        } else {
            params.eq_enabled.store(false, Relaxed);
        }

        if let Some(dly) = &self.delay {
            params
                .delay_enabled
                .store(dly.enabled.unwrap_or(true), Relaxed);
            params.delay_time.store(dly.time.clamp(0.0, 1.0), Relaxed);
            params
                .delay_feedback
                .store(dly.feedback.clamp(0.0, 1.0), Relaxed);
            params.delay_mix.store(dly.mix.clamp(0.0, 1.0), Relaxed);
        } else {
            params.delay_enabled.store(false, Relaxed);
        }

        if let Some(fl) = &self.flanger {
            params.fl_enabled.store(fl.enabled.unwrap_or(true), Relaxed);
            params.fl_rate.store(fl.rate.clamp(0.0, 1.0), Relaxed);
            params.fl_depth.store(fl.depth.clamp(0.0, 1.0), Relaxed);
            params
                .fl_feedback
                .store(fl.feedback.clamp(0.0, 1.0), Relaxed);
            params.fl_mix.store(fl.mix.clamp(0.0, 1.0), Relaxed);
        } else {
            params.fl_enabled.store(false, Relaxed);
        }

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

pub fn find_preset_files() -> Vec<(PathBuf, PresetSource)> {
    let system_dir = PathBuf::from("presets");
    let mut result: Vec<(PathBuf, PresetSource)> = Vec::new();

    let scan = |dir: &PathBuf, source: PresetSource| -> Vec<(PathBuf, PresetSource)> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut files: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
                .collect();
            files.sort();
            files.into_iter().map(|p| (p, source)).collect()
        } else {
            vec![]
        }
    };

    result.extend(scan(&system_dir, PresetSource::System));

    if let Some(home) = dirs::home_dir() {
        let user_dir = home.join(".config").join("rusty-amp").join("presets");
        result.extend(scan(&user_dir, PresetSource::User));
    }

    result
}

fn load_embedded() -> Vec<Preset> {
    let mut names: Vec<String> = BundledPresets::iter().map(|n| n.into_owned()).collect();
    names.sort();
    names
        .into_iter()
        .filter_map(|name| {
            let file = BundledPresets::get(&name)?;
            let src = std::str::from_utf8(file.data.as_ref()).ok()?;
            let mut preset: Preset = toml::from_str(src)
                .map_err(|e| eprintln!("Warning: skipping embedded preset {name}: {e}"))
                .ok()?;
            preset.source = PresetSource::System;
            preset.path = None;
            Some(preset)
        })
        .collect()
}

pub fn load_all() -> Vec<Preset> {
    let from_disk: Vec<Preset> = find_preset_files()
        .into_iter()
        .filter_map(|(path, source)| {
            Preset::load(&path, source)
                .map_err(|e| eprintln!("Warning: skipping preset {}: {e}", path.display()))
                .ok()
        })
        .collect();

    // If the ./presets/ directory is absent (installed binary), fall back to embedded.
    let system_on_disk = from_disk.iter().any(|p| p.source == PresetSource::System);
    if system_on_disk {
        from_disk
    } else {
        let mut all = load_embedded();
        all.extend(from_disk);
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every bundled preset must deserialize against the current schema — guards
    /// against a typo or a renamed field silently breaking a shipped preset.
    #[test]
    fn all_bundled_presets_parse() {
        let mut count = 0;
        for entry in std::fs::read_dir("presets").expect("presets/ dir") {
            let p = entry.unwrap().path();
            if p.extension().is_some_and(|ext| ext == "toml") {
                Preset::load(&p, PresetSource::System)
                    .unwrap_or_else(|e| panic!("failed to parse {}: {e}", p.display()));
                count += 1;
            }
        }
        assert!(count > 0, "no bundled presets found to validate");
    }
}
