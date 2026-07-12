pub use rand::{rngs::SmallRng, SeedableRng};

pub type Rng = SmallRng;
pub const DEFAULT_SEED: u64 = 123;