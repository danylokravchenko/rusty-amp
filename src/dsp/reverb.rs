//! Freeverb mono reverb by Jezar at Dreampoint (public domain algorithm).
//!
//! Architecture: input × gain → 8 parallel comb filters (summed) → 4 series allpass filters
//! Comb filters provide the dense reflections; allpass filters diffuse them
const FIXED_GAIN: f32 = 0.015;
const SCALE_WET: f32 = 3.0;
const SCALE_ROOM: f32 = 0.28;
const OFFSET_ROOM: f32 = 0.7;
const SCALE_DAMP: f32 = 0.4;
const ALLPASS_FEEDBACK: f32 = 0.5;

// Delay lengths tuned at 44100 Hz; scaled proportionally for other rates.
const COMB_DELAYS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_DELAYS: [usize; 4] = [556, 441, 341, 225];

// ── Comb filter ──────────────────────────────────────────────────────────────

struct CombFilter {
    buf: Vec<f32>,
    pos: usize,
    feedback: f32,
    damp1: f32, // high-frequency damping coefficient
    damp2: f32, // complement of damp1
    filterstore: f32,
}

impl CombFilter {
    fn new(size: usize) -> Self {
        Self {
            buf: vec![0.0; size],
            pos: 0,
            feedback: 0.84,
            damp1: 0.2,
            damp2: 0.8,
            filterstore: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let out = self.buf[self.pos];
        // One-pole low-pass on the feedback path (simulates air absorption)
        self.filterstore = out * self.damp2 + self.filterstore * self.damp1;
        self.buf[self.pos] = input + self.filterstore * self.feedback;
        self.pos = (self.pos + 1) % self.buf.len();
        out
    }

    fn set_params(&mut self, room: f32, damp: f32) {
        self.feedback = room;
        self.damp1 = damp;
        self.damp2 = 1.0 - damp;
    }
}

// ── Allpass filter ───────────────────────────────────────────────────────────

struct AllpassFilter {
    buf: Vec<f32>,
    pos: usize,
}

impl AllpassFilter {
    fn new(size: usize) -> Self {
        Self {
            buf: vec![0.0; size],
            pos: 0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let bufout = self.buf[self.pos];
        let output = -input + bufout;
        self.buf[self.pos] = input + bufout * ALLPASS_FEEDBACK;
        self.pos = (self.pos + 1) % self.buf.len();
        output
    }
}

// ── Public reverb struct ─────────────────────────────────────────────────────

pub struct Reverb {
    combs: [CombFilter; 8],
    allpasses: [AllpassFilter; 4],
    last_room: f32,
    last_damp: f32,
}

impl Reverb {
    pub fn new(sr: f32) -> Self {
        let scale = sr / 44100.0;
        let combs = COMB_DELAYS.map(|d| CombFilter::new(((d as f32 * scale) as usize).max(1)));
        let allpasses =
            ALLPASS_DELAYS.map(|d| AllpassFilter::new(((d as f32 * scale) as usize).max(1)));
        let mut r = Self {
            combs,
            allpasses,
            last_room: -1.0,
            last_damp: -1.0,
        };
        r.update_params(0.5, 0.4);
        r
    }

    fn update_params(&mut self, room: f32, damp: f32) {
        let room_size = room * SCALE_ROOM + OFFSET_ROOM;
        let damp_scaled = damp * SCALE_DAMP;
        for c in &mut self.combs {
            c.set_params(room_size, damp_scaled);
        }
        self.last_room = room;
        self.last_damp = damp;
    }

    /// `room` 0–1, `damp` 0–1, `mix` 0–1 (dry/wet)
    #[inline]
    pub fn process(&mut self, dry: f32, room: f32, damp: f32, mix: f32) -> f32 {
        if (room - self.last_room).abs() > 0.001 || (damp - self.last_damp).abs() > 0.001 {
            self.update_params(room, damp);
        }

        let input = dry * FIXED_GAIN;
        let mut wet = 0.0f32;
        for c in &mut self.combs {
            wet += c.process(input);
        }
        for ap in &mut self.allpasses {
            wet = ap.process(wet);
        }

        dry * (1.0 - mix) + wet * SCALE_WET * mix
    }
}
