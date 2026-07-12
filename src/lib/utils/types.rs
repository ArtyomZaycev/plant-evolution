#[cfg(feature = "stable_rng")]
mod stable_rng {
    use rand::{rngs::SmallRng, SeedableRng};

    pub type Rng = SmallRng;
    pub const DEFAULT_SEED: u64 = 123;

    pub fn get_rng() -> Rng {
        SmallRng::seed_from_u64(DEFAULT_SEED)
    }
}

#[cfg(not(feature = "stable_rng"))]
mod unstable_rng {
    use rand::rngs::ThreadRng;

    pub type Rng = ThreadRng;
    pub const DEFAULT_SEED: u64 = 123;

    pub fn get_rng() -> Rng {
        rand::rng()
    }
}

#[cfg(feature = "stable_rng")]
pub use stable_rng::*;
#[cfg(not(feature = "stable_rng"))]
pub use unstable_rng::*;