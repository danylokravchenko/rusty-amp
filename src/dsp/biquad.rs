use std::f32::consts::PI;

/// Second-order IIR filter, Direct Form II Transposed.
/// Coefficients follow the Audio EQ Cookbook by Robert Bristow-Johnson.
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn from_coeffs(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub fn highpass(sr: f32, freq: f32, q: f32) -> Self {
        let w0 = 2.0 * PI * freq / sr;
        let (s, c) = (w0.sin(), w0.cos());
        let alpha = s / (2.0 * q);
        Self::from_coeffs(
            (1.0 + c) / 2.0,
            -(1.0 + c),
            (1.0 + c) / 2.0,
            1.0 + alpha,
            -2.0 * c,
            1.0 - alpha,
        )
    }

    #[allow(dead_code)]
    pub fn lowpass(sr: f32, freq: f32, q: f32) -> Self {
        let w0 = 2.0 * PI * freq / sr;
        let (s, c) = (w0.sin(), w0.cos());
        let alpha = s / (2.0 * q);
        Self::from_coeffs(
            (1.0 - c) / 2.0,
            1.0 - c,
            (1.0 - c) / 2.0,
            1.0 + alpha,
            -2.0 * c,
            1.0 - alpha,
        )
    }

    pub fn low_shelf(sr: f32, freq: f32, gain_db: f32) -> Self {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sr;
        let (s, c) = (w0.sin(), w0.cos());
        let alpha = s / 2.0 * 2.0_f32.sqrt();
        let sq = 2.0 * a.sqrt() * alpha;
        Self::from_coeffs(
            a * ((a + 1.0) - (a - 1.0) * c + sq),
            2.0 * a * ((a - 1.0) - (a + 1.0) * c),
            a * ((a + 1.0) - (a - 1.0) * c - sq),
            (a + 1.0) + (a - 1.0) * c + sq,
            -2.0 * ((a - 1.0) + (a + 1.0) * c),
            (a + 1.0) + (a - 1.0) * c - sq,
        )
    }

    pub fn high_shelf(sr: f32, freq: f32, gain_db: f32) -> Self {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sr;
        let (s, c) = (w0.sin(), w0.cos());
        let alpha = s / 2.0 * 2.0_f32.sqrt();
        let sq = 2.0 * a.sqrt() * alpha;
        Self::from_coeffs(
            a * ((a + 1.0) + (a - 1.0) * c + sq),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * c),
            a * ((a + 1.0) + (a - 1.0) * c - sq),
            (a + 1.0) - (a - 1.0) * c + sq,
            2.0 * ((a - 1.0) - (a + 1.0) * c),
            (a + 1.0) - (a - 1.0) * c - sq,
        )
    }

    pub fn peak_eq(sr: f32, freq: f32, q: f32, gain_db: f32) -> Self {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sr;
        let (s, c) = (w0.sin(), w0.cos());
        let alpha = s / (2.0 * q);
        Self::from_coeffs(
            1.0 + alpha * a,
            -2.0 * c,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * c,
            1.0 - alpha / a,
        )
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}
