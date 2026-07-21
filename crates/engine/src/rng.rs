//! Derive-per-op PRNG: each random operation gets its own stream keyed from the master seed.

/// Splitmix64 state — a tiny, well-distributed deterministic PRNG.
pub struct OpRng {
    state: u64,
}

impl OpRng {
    pub fn from_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Unbiased index in `0..upper_exclusive`. Panics if `upper_exclusive == 0`.
    pub fn gen_index(&mut self, upper_exclusive: usize) -> usize {
        assert!(upper_exclusive > 0);
        let upper = upper_exclusive as u64;
        let thresh = upper.wrapping_neg() % upper;
        loop {
            let r = self.next_u64();
            if r >= thresh {
                return (r % upper) as usize;
            }
        }
    }
}

pub fn derive_op_key(master_seed: &[u8; 32], player: u8, iteration: u64) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(master_seed);
    h.update(&[player]);
    h.update(&iteration.to_le_bytes());
    *h.finalize().as_bytes()
}

pub fn op_rng_from_key(key: &[u8; 32]) -> OpRng {
    let mut seed_bytes = [0u8; 8];
    seed_bytes.copy_from_slice(&key[..8]);
    OpRng::from_seed(u64::from_le_bytes(seed_bytes))
}
