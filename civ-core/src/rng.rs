//! Deterministic random numbers (SplitMix64). The world owns one sequential stream; everything
//! is single-threaded in band logic so results are reproducible per seed.
#[derive(Clone, Debug)]
pub struct Rng(pub(crate) u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut r = Rng(seed ^ 0x9E37_79B9_7F4A_7C15);
        r.next_u64();
        r
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[inline]
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 * (1.0 / (1u64 << 24) as f32)
    }
    #[inline]
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + self.f32() * (b - a)
    }
    #[inline]
    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    /// Cheap pseudo-gaussian, mean 0, sd ~1.
    #[inline]
    pub fn gauss(&mut self) -> f32 {
        (self.f32() + self.f32() + self.f32() - 1.5) * 2.0
    }
    /// Poisson sample (Knuth; fine for small lambda).
    pub fn poisson(&mut self, lambda: f32) -> u32 {
        if lambda <= 0.0 {
            return 0;
        }
        let l = (-lambda.min(30.0)).exp();
        let (mut k, mut p) = (0u32, 1.0f32);
        loop {
            p *= self.f32();
            if p <= l {
                return k;
            }
            k += 1;
        }
    }
    /// Pick an index with probability proportional to weights (None if all zero).
    pub fn weighted(&mut self, w: &[f32]) -> Option<usize> {
        let s: f32 = w.iter().sum();
        if s <= 0.0 {
            return None;
        }
        let mut x = self.f32() * s;
        for (i, &v) in w.iter().enumerate() {
            if x < v {
                return Some(i);
            }
            x -= v;
        }
        Some(w.len() - 1)
    }
}

/// Stateless hash used by noise and map generation.
#[inline]
pub fn hash3(seed: u64, x: i64, y: i64) -> u64 {
    let mut z = seed
        ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
