pub mod marshall;
pub mod mesa;
pub mod randall;

use crate::dsp::AmpModel;

pub use marshall::Marshall;
pub use mesa::Mesa;
pub use randall::Randall;

// ── Dynamic "bloom" ─────────────────────────────────────────────────────────

/// Slow envelope follower used to give a gain stage playing dynamics.
///
/// A real tube's operating point drifts under sustained drive (grid-bias
/// excursion / cathode self-bias). Feeding this envelope in as a small DC bias
/// before an asymmetric waveshaper increases even-harmonic content and adds a
/// gentle "give" the harder you play — the touch sensitivity and bloom that make
/// a tube amp feel alive rather than statically clamped. The following
/// inter-stage high-pass removes the injected DC, leaving only the harmonic and
/// compression effect.
pub(crate) struct Bloom {
    env: f32,
    atk: f32,
    rel: f32,
}

impl Bloom {
    pub fn new(sr: f32, atk_ms: f32, rel_ms: f32) -> Self {
        Self {
            env: 0.0,
            atk: 1.0 - (-1.0 / (atk_ms * 0.001 * sr)).exp(),
            rel: 1.0 - (-1.0 / (rel_ms * 0.001 * sr)).exp(),
        }
    }

    #[inline]
    pub fn follow(&mut self, x: f32) -> f32 {
        let a = x.abs();
        let c = if a > self.env { self.atk } else { self.rel };
        self.env += c * (a - self.env);
        self.env
    }
}

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
