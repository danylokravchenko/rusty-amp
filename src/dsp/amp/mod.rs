pub mod marshall;
pub mod mesa;
pub mod randall;

use crate::dsp::AmpModel;

pub use marshall::Marshall;
pub use mesa::Mesa;
pub use randall::Randall;

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Common interface every amp model must satisfy.
/// All knobs are normalised 0–1.
pub trait Amplifier {
    #[allow(clippy::too_many_arguments)]
    fn process(
        &mut self,
        sample: f32,
        gain: f32,
        bass: f32,
        mid: f32,
        treble: f32,
        presence: f32,
        master: f32,
    ) -> f32;
}

// ── Bank ──────────────────────────────────────────────────────────────────────

/// Owns all amp instances simultaneously so filter state is preserved across
/// model switches (no audible click from zeroed delay lines on switch).
pub struct AmpBank {
    marshall: Marshall,
    mesa: Mesa,
    randall: Randall,
}

impl AmpBank {
    pub fn new(sr: f32) -> Self {
        Self {
            marshall: Marshall::new(sr),
            mesa: Mesa::new(sr),
            randall: Randall::new(sr),
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn process(
        &mut self,
        model: AmpModel,
        sample: f32,
        gain: f32,
        bass: f32,
        mid: f32,
        treble: f32,
        presence: f32,
        master: f32,
    ) -> f32 {
        match model {
            AmpModel::Marshall => self
                .marshall
                .process(sample, gain, bass, mid, treble, presence, master),
            AmpModel::Mesa => self
                .mesa
                .process(sample, gain, bass, mid, treble, presence, master),
            AmpModel::Randall => self
                .randall
                .process(sample, gain, bass, mid, treble, presence, master),
        }
    }
}
