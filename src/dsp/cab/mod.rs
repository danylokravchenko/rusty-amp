pub mod marshall;
pub mod mesa;

pub use marshall::MarshallCab;
pub use mesa::MesaCab;

// ── Trait ─────────────────────────────────────────────────────────────────────

pub trait Cabinet {
    fn process(&mut self, sample: f32, mic_pos: f32) -> f32;
}

// ── Bank ──────────────────────────────────────────────────────────────────────

/// Owns all cabinet instances simultaneously so filter state survives model switches.
pub struct CabBank {
    mesa: MesaCab,
    marshall: MarshallCab,
}

impl CabBank {
    pub fn new(sr: f32) -> Self {
        Self {
            mesa: MesaCab::new(sr),
            marshall: MarshallCab::new(sr),
        }
    }

    #[inline]
    pub fn process(&mut self, model: super::CabModel, sample: f32, mic_pos: f32) -> f32 {
        match model {
            super::CabModel::Mesa => self.mesa.process(sample, mic_pos),
            super::CabModel::Marshall => self.marshall.process(sample, mic_pos),
        }
    }
}
