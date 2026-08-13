use rand::{SeedableRng, rngs::SmallRng};

pub type Rng = SmallRng;

#[cfg(feature = "stable_rng")]
const DEFAULT_SEED: u64 = 844311;

#[cfg(feature = "stable_rng")]
pub fn get_seed() -> u64 {
    DEFAULT_SEED
}

#[cfg(not(feature = "stable_rng"))]
pub fn get_seed() -> u64 {
    use rand::Rng;

    let mut rng = rand::rng();
    rng.next_u64() % 1_000_033
}

pub fn get_rng() -> Rng {
    get_rng_seeded(get_seed())
}

pub fn get_rng_seeded(seed: u64) -> Rng {
    SmallRng::seed_from_u64(seed)
}