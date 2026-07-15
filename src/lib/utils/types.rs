#[cfg(feature = "stable_rng")]
mod rng {
    use rand::{SeedableRng, rngs::SmallRng};

    pub type Rng = SmallRng;
    pub const DEFAULT_SEED: u64 = 8867;

    pub fn get_rng() -> Rng {
        SmallRng::seed_from_u64(DEFAULT_SEED)
    }
}

#[cfg(not(feature = "stable_rng"))]
mod rng {
    use rand::rngs::ThreadRng;

    pub type Rng = ThreadRng;

    pub fn get_rng() -> Rng {
        rand::rng()
    }
}

pub use rng::*;

pub const DEFAULT_THREAD_COUNT: u32 = 8;
