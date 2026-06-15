use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

use crate::utils::Rng;

use super::{
    parents_evolution::*,
    random_evolution::*,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithVolatility<T> {
    pub value: T,
    pub volatility: f32,
}

impl<T> WithVolatility<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            volatility: 1.,
        }
    }
}

impl<T> Deref for WithVolatility<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for WithVolatility<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

// Evolution with volatility parameter
impl<T: RandomEvolution> RandomEvolution for WithVolatility<T> {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) -> bool {
        let changed = self.value.evolve_random(
            rng,
            (change_chance * self.volatility).clamp(0.05, 0.9),
            change_entropy,
        );
        self.volatility *= if changed { 1.1 } else { 0.999 };
        self.volatility = self.volatility.clamp(0.1, 2.);
        changed
    }
}

impl<T: ParentCombination> ParentCombination for WithVolatility<T> {
    fn parent_combine(&self, rng: &mut Rng, other: &Self) -> Self {
        Self {
            value: self.value.parent_combine(rng, &other.value),
            volatility: (self.volatility + other.volatility) / 2.,
        }
    }
}
