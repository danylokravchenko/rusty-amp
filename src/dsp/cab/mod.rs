pub mod ir;
pub mod marshall;
pub mod mesa;
pub mod orange;

use crate::dsp::biquad::Biquad;
use crate::dsp::conv::FftConvolver;

pub use marshall::MarshallCab;
pub use mesa::MesaCab;
pub use orange::OrangeCab;

// ── Trait ─────────────────────────────────────────────────────────────────────

pub trait Cabinet {
    /// Convolve a mono amp sample with the cab IR, returning a stereo (L, R) pair.
    ///
    /// `mic_pos` moves the close mic edge→centre, `blend` crossfades the close
    /// dynamic (SM57) into a ribbon (R121), and `room` adds an ambient room mic.
    fn process(&mut self, sample: f32, mic_pos: f32, blend: f32, room: f32) -> (f32, f32);
}

// ── Blended multi-mic cabinet ──────────────────────────────────────────────────

/// A studio "mic'd cab" rendered from three mic captures per channel, blended:
///   • a close dynamic (SM57) — bright, present, the backbone of the tone;
///   • a close ribbon (R121) — darker, smoother top, fuller low-mids;
///   • a room mic — distant, ambient, adds depth and air.
///
/// Each capture is a full impulse response (its own voicing + reflection texture,
/// with the room mic carrying extra pre-delay and denser late reflections). Because
/// convolution is linear, blending the mics is just a weighted **sum of their IRs**:
/// we precompute the three IRs once and, whenever a blend knob moves, recombine
/// them into the live convolver's taps. The per-sample cost is therefore exactly
/// two convolutions regardless of how many mics are in the blend, and swapping the
/// taps preserves the delay-line history so it never clicks.
pub struct BlendedCab {
    sr: f32,
    conv_l: FftConvolver,
    conv_r: FftConvolver,
    // Per-mic impulse responses (close / ribbon / room), per channel.
    close_l: Vec<f32>,
    close_r: Vec<f32>,
    ribbon_l: Vec<f32>,
    ribbon_r: Vec<f32>,
    room_l: Vec<f32>,
    room_r: Vec<f32>,
    // Preallocated combine buffers so `process` never allocates.
    scratch_l: Vec<f32>,
    scratch_r: Vec<f32>,
    // Close-mic position shelf (edge↔centre).
    mic_l: Biquad,
    mic_r: Biquad,
    last_mic_pos: f32,
    last_blend: f32,
    last_room: f32,
}

impl BlendedCab {
    /// Build from the six prebuilt IRs: `[close_l, close_r, ribbon_l, ribbon_r,
    /// room_l, room_r]`.
    pub fn new(sr: f32, irs: [Vec<f32>; 6]) -> Self {
        let [close_l, close_r, ribbon_l, ribbon_r, room_l, room_r] = irs;
        let cap = close_l.len() + 1;
        let mut cab = Self {
            sr,
            conv_l: FftConvolver::new(cap),
            conv_r: FftConvolver::new(cap),
            scratch_l: vec![0.0; close_l.len()],
            scratch_r: vec![0.0; close_r.len()],
            close_l,
            close_r,
            ribbon_l,
            ribbon_r,
            room_l,
            room_r,
            mic_l: Biquad::high_shelf(sr, 5000.0, 0.0),
            mic_r: Biquad::high_shelf(sr, 5000.0, 0.0),
            last_mic_pos: -1.0,
            last_blend: -1.0,
            last_room: -1.0,
        };
        cab.recombine(0.0, 0.0);
        cab
    }

    /// Mix the three mic IRs into the convolver taps for a given blend.
    /// `blend` 0 = close dynamic … 1 = ribbon; `room` 0–1 = ambient room amount.
    fn recombine(&mut self, blend: f32, room: f32) {
        let wc = 1.0 - blend; // close dynamic weight
        let wr = blend; // ribbon weight
        let wroom = room * 0.6; // room is additive ambience, kept lower
        for (i, s) in self.scratch_l.iter_mut().enumerate() {
            *s = wc * self.close_l[i] + wr * self.ribbon_l[i] + wroom * self.room_l[i];
        }
        for (i, s) in self.scratch_r.iter_mut().enumerate() {
            *s = wc * self.close_r[i] + wr * self.ribbon_r[i] + wroom * self.room_r[i];
        }
        self.conv_l.load(&self.scratch_l);
        self.conv_r.load(&self.scratch_r);
        self.last_blend = blend;
        self.last_room = room;
    }

    #[inline]
    pub fn process(&mut self, sample: f32, mic_pos: f32, blend: f32, room: f32) -> (f32, f32) {
        if (blend - self.last_blend).abs() > 0.001 || (room - self.last_room).abs() > 0.001 {
            self.recombine(blend, room);
        }
        if (mic_pos - self.last_mic_pos).abs() > 0.001 {
            // 0 = edge (off-axis, dark), 1 = centre (on-axis, bright)
            let db = (mic_pos - 0.5) * 12.0;
            self.mic_l = Biquad::high_shelf(self.sr, 5000.0, db);
            self.mic_r = Biquad::high_shelf(self.sr, 5000.0, db);
            self.last_mic_pos = mic_pos;
        }
        let l = self.mic_l.process(self.conv_l.process(sample));
        let r = self.mic_r.process(self.conv_r.process(sample));
        (l, r)
    }
}

// ── Bank ──────────────────────────────────────────────────────────────────────

/// Owns all cabinet instances simultaneously so filter state survives model switches.
pub struct CabBank {
    mesa: MesaCab,
    marshall: MarshallCab,
    orange: OrangeCab,
}

impl CabBank {
    pub fn new(sr: f32) -> Self {
        Self {
            mesa: MesaCab::new(sr),
            marshall: MarshallCab::new(sr),
            orange: OrangeCab::new(sr),
        }
    }

    #[inline]
    pub fn process(
        &mut self,
        model: super::CabModel,
        sample: f32,
        mic_pos: f32,
        blend: f32,
        room: f32,
    ) -> (f32, f32) {
        match model {
            super::CabModel::Mesa => self.mesa.process(sample, mic_pos, blend, room),
            super::CabModel::Marshall => self.marshall.process(sample, mic_pos, blend, room),
            super::CabModel::Orange => self.orange.process(sample, mic_pos, blend, room),
        }
    }
}
