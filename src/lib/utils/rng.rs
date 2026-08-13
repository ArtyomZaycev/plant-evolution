use rand::{SeedableRng, rngs::SmallRng};

pub type Rng = SmallRng;

#[cfg(feature = "stable_rng")]
const DEFAULT_SEED: u64 = 844311;
#[cfg(not(feature = "stable_rng"))]
const SEED_ENV: &str = "SEED";

#[cfg(feature = "stable_rng")]
pub fn get_seed() -> u64 {
    DEFAULT_SEED
}

#[cfg(not(feature = "stable_rng"))]
/// Same value within one run
pub fn get_seed() -> u64 {
    if let Some(seed) = std::env::var(SEED_ENV)
        .ok()
        .and_then(|seed| seed.parse::<u64>().ok())
    {
        seed
    } else {
        let seed = get_random_seed();
        unsafe {
            std::env::set_var(SEED_ENV, seed.to_string());
        }
        seed
    }
}

pub fn get_random_seed() -> u64 {
    rand::random::<u64>() % 1_000_033
}

pub fn get_rng() -> Rng {
    get_rng_seeded(get_seed())
}

pub fn get_rng_seeded(seed: u64) -> Rng {
    SmallRng::seed_from_u64(seed)
}
