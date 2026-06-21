//! Passive FMV ("Fender/Marshall/Vox") tone stack.
//!
//! The previous amp tone controls were three independent biquads (a low shelf, a
//! mid peak and a high shelf). That is convenient but wrong: in a real amp the
//! Bass/Mid/Treble pots sit in **one** passive RC network and interact strongly —
//! turning treble up pulls the mids down, the mid pot sets the depth of the
//! ever-present mid scoop, and the whole stack is lossy (it only ever attenuates).
//! That interaction and the characteristic mid dip are a huge part of why a JCM800
//! or a Rectifier sounds the way it does.
//!
//! This models the actual analog transfer function of the FMV stack (the classic
//! 3rd-order network analysed by David Yeh, DAFx-06) and discretises it with the
//! bilinear transform. The result is a single 3rd-order IIR whose coefficients are
//! recomputed only when a knob moves.
//!
//! Because a passive stack always attenuates, the response is peak-normalised to
//! unity (`makeup`) so the amp's gain staging is preserved — the stack colours the
//! tone without changing the operating level the rest of the amp was tuned around.

use std::f32::consts::PI;

/// Resistor/capacitor values for one FMV stack.
#[derive(Clone, Copy)]
pub struct Components {
    pub r1: f32,
    pub r2: f32,
    pub r3: f32,
    pub r4: f32,
    pub c1: f32,
    pub c2: f32,
    pub c3: f32,
}

impl Components {
    /// Marshall JCM800 tone stack (bright, pronounced mid scoop).
    pub const MARSHALL: Components = Components {
        r1: 250e3,
        r2: 1e6,
        r3: 25e3,
        r4: 56e3,
        c1: 470e-12,
        c2: 22e-9,
        c3: 22e-9,
    };

    /// Fender-style stack (used for the Mesa — fuller lows, gentler scoop than the
    /// Marshall, which suits the Rectifier's thicker voicing).
    pub const FENDER: Components = Components {
        r1: 250e3,
        r2: 250e3,
        r3: 10e3,
        r4: 100e3,
        c1: 250e-12,
        c2: 100e-9,
        c3: 47e-9,
    };
}

pub struct ToneStack {
    sr: f32,
    comp: Components,
    // z-domain coefficients (a0 normalised to 1).
    b0: f32,
    b1: f32,
    b2: f32,
    b3: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    makeup: f32,
    // Direct Form I state.
    x1: f32,
    x2: f32,
    x3: f32,
    y1: f32,
    y2: f32,
    y3: f32,
}

impl ToneStack {
    pub fn new(sr: f32, comp: Components) -> Self {
        let mut ts = Self {
            sr,
            comp,
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            b3: 0.0,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            makeup: 1.0,
            x1: 0.0,
            x2: 0.0,
            x3: 0.0,
            y1: 0.0,
            y2: 0.0,
            y3: 0.0,
        };
        ts.update(0.5, 0.5, 0.5);
        ts
    }

    /// Recompute the filter for new pot positions (each 0–1). Audio-taper-ish
    /// curves are applied so the controls track a real amp's feel.
    pub fn update(&mut self, bass: f32, mid: f32, treble: f32) {
        // Pots: clamp off the rails (a pot at exactly 0 makes the network
        // degenerate). Bass uses an audio-ish taper; treble/mid stay near-linear.
        let l = (bass * bass).clamp(0.001, 0.999);
        let m = mid.clamp(0.001, 0.999);
        let t = treble.clamp(0.001, 0.999);

        let Components {
            r1,
            r2,
            r3,
            r4,
            c1,
            c2,
            c3,
        } = self.comp;

        // ── Analog transfer function H(s) = (b1·s + b2·s² + b3·s³) / (a0 + a1·s +
        //    a2·s² + a3·s³), coefficients per Yeh's FMV analysis. ───────────────
        let b1 = t * c1 * r1 + m * c3 * r3 + l * (c1 * r2 + c2 * r2);
        let b2 = t * (c1 * c2 * r1 * r4 + c1 * c3 * r1 * r4)
            - m * m * (c1 * c3 * r3 * r3 + c2 * c3 * r3 * r3)
            + m * (c1 * c3 * r1 * r3 + c1 * c3 * r3 * r3 + c2 * c3 * r3 * r3)
            + l * (c1 * c2 * r1 * r2 + c1 * c2 * r2 * r4 + c1 * c3 * r2 * r4)
            + l * m * (c1 * c3 * r2 * r3 + c2 * c3 * r2 * r3);
        let b3 = l * m * (c1 * c2 * c3 * r1 * r2 * r3 + c1 * c2 * c3 * r2 * r3 * r4)
            - m * m * (c1 * c2 * c3 * r1 * r3 * r3 + c1 * c2 * c3 * r3 * r3 * r4)
            + m * (c1 * c2 * c3 * r1 * r3 * r3 + c1 * c2 * c3 * r3 * r3 * r4)
            + t * c1 * c2 * c3 * r1 * r3 * r4
            - t * m * c1 * c2 * c3 * r1 * r3 * r4
            + t * l * c1 * c2 * c3 * r1 * r2 * r4;

        let a0 = 1.0;
        let a1 = (c1 * r1 + c1 * r3 + c2 * r3 + c2 * r4 + c3 * r4)
            + m * c3 * r3
            + l * (c1 * r2 + c2 * r2);
        let a2 = m
            * (c1 * c3 * r1 * r3 - c2 * c3 * r3 * r4 + c1 * c3 * r3 * r3 + c2 * c3 * r3 * r3)
            - m * m * (c1 * c3 * r3 * r3 + c2 * c3 * r3 * r3)
            + l * (c1 * c2 * r1 * r2 + c1 * c2 * r2 * r4 + c1 * c3 * r2 * r4 + c2 * c3 * r2 * r4)
            + l * m * (c1 * c3 * r2 * r3 + c2 * c3 * r2 * r3)
            + (c1 * c2 * r1 * r4
                + c1 * c3 * r1 * r4
                + c1 * c2 * r3 * r4
                + c1 * c2 * r1 * r3
                + c1 * c3 * r3 * r4
                + c2 * c3 * r3 * r4);
        let a3 = l * m * (c1 * c2 * c3 * r1 * r2 * r3 + c1 * c2 * c3 * r2 * r3 * r4)
            - m * m * (c1 * c2 * c3 * r1 * r3 * r3 + c1 * c2 * c3 * r3 * r3 * r4)
            + m * (c1 * c2 * c3 * r3 * r3 * r4 + c1 * c2 * c3 * r1 * r3 * r3
                - c1 * c2 * c3 * r1 * r3 * r4)
            + l * c1 * c2 * c3 * r1 * r2 * r4
            + c1 * c2 * c3 * r1 * r3 * r4;

        // ── Bilinear transform: s = c·(1 − z⁻¹)/(1 + z⁻¹), c = 2·fs. ───────────
        // Multiplying num/den by (1 + z⁻¹)³ and collecting powers of z⁻¹ gives the
        // four z-coefficients for each cubic polynomial.
        let c = 2.0 * self.sr;
        let c2v = c * c;
        let c3v = c2v * c;

        // Numerator: p0 = 0, p1 = b1, p2 = b2, p3 = b3.
        let bz0 = b1 * c + b2 * c2v + b3 * c3v;
        let bz1 = b1 * c - b2 * c2v - 3.0 * b3 * c3v;
        let bz2 = -b1 * c - b2 * c2v + 3.0 * b3 * c3v;
        let bz3 = -b1 * c + b2 * c2v - b3 * c3v;

        // Denominator: p0 = a0, p1 = a1, p2 = a2, p3 = a3.
        let az0 = a0 + a1 * c + a2 * c2v + a3 * c3v;
        let az1 = 3.0 * a0 + a1 * c - a2 * c2v - 3.0 * a3 * c3v;
        let az2 = 3.0 * a0 - a1 * c - a2 * c2v + 3.0 * a3 * c3v;
        let az3 = a0 - a1 * c + a2 * c2v - a3 * c3v;

        let inv = 1.0 / az0;
        self.b0 = bz0 * inv;
        self.b1 = bz1 * inv;
        self.b2 = bz2 * inv;
        self.b3 = bz3 * inv;
        self.a1 = az1 * inv;
        self.a2 = az2 * inv;
        self.a3 = az3 * inv;

        // Peak-normalise: a passive stack only cuts, so scale the response so its
        // loudest band is unity. This keeps the amp's downstream gain staging put.
        self.makeup = 1.0 / self.peak_magnitude().max(1e-6);
    }

    /// Largest |H(e^{jω})| over a log-spaced sweep of the audio band.
    fn peak_magnitude(&self) -> f32 {
        let mut peak = 0.0f32;
        let mut f = 20.0f32;
        while f < 18_000.0 {
            let w = 2.0 * PI * f / self.sr;
            let (c1, s1) = (w.cos(), w.sin());
            let (c2, s2) = ((2.0 * w).cos(), (2.0 * w).sin());
            let (c3, s3) = ((3.0 * w).cos(), (3.0 * w).sin());
            // Numerator / denominator evaluated at e^{-jω}.
            let nr = self.b0 + self.b1 * c1 + self.b2 * c2 + self.b3 * c3;
            let ni = -(self.b1 * s1 + self.b2 * s2 + self.b3 * s3);
            let dr = 1.0 + self.a1 * c1 + self.a2 * c2 + self.a3 * c3;
            let di = -(self.a1 * s1 + self.a2 * s2 + self.a3 * s3);
            let mag = ((nr * nr + ni * ni) / (dr * dr + di * di)).sqrt();
            peak = peak.max(mag);
            f *= 1.10;
        }
        peak
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 + self.b3 * self.x3
            - self.a1 * self.y1
            - self.a2 * self.y2
            - self.a3 * self.y3;
        self.x3 = self.x2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y3 = self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y * self.makeup
    }
}
