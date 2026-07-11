use rand::RngExt;

use super::WeightsTree;
use crate::{evolution::evolution::*, map::*, utils::*};

fn choose_random<T: Clone>(rng: &mut Rng, v1: &T, v2: &T) -> T {
    if rng.random_bool(0.5) {
        v1.clone()
    } else {
        v2.clone()
    }
}

pub trait ParentCombination {
    fn parent_combine(&self, rng: &mut Rng, other: &Self) -> Self;
}

impl<T: ParentCombination> ParentCombination for Vec<T> {
    fn parent_combine(&self, rng: &mut Rng, other: &Self) -> Self {
        assert!(self.len() == other.len());
        self.iter()
            .enumerate()
            .map(|(i, s)| s.parent_combine(rng, &other[i]))
            .collect()
    }
}

impl<T: ParentCombination, const N: usize> ParentCombination for [T; N] {
    fn parent_combine(&self, rng: &mut Rng, other: &Self) -> Self {
        std::array::from_fn(|i| self[i].parent_combine(rng, &other[i]))
    }
}

// Evolution with volatility parameter
impl<T: ParentCombination> ParentCombination for (T, f32) {
    fn parent_combine(&self, rng: &mut Rng, other: &Self) -> Self {
        (
            self.0.parent_combine(rng, &other.0),
            (self.1 + other.1) / 2.,
        )
    }
}

impl ParentCombination for PlantEvolutionData {
    fn parent_combine(&self, rng: &mut Rng, other: &Self) -> Self {
        Self {
            evolutions: self.evolutions,
            cells_evolution_data: self
                .cells_evolution_data
                .parent_combine(rng, &other.cells_evolution_data),
            cells_abilities: self
                .cells_abilities
                .parent_combine(rng, &other.cells_abilities),
        }
    }
}

impl ParentCombination for CellEvolutionData {
    fn parent_combine(&self, rng: &mut Rng, other: &Self) -> Self {
        Self {
            weights: self.weights.parent_combine(rng, &other.weights),
            suicide_weights: self
                .suicide_weights
                .parent_combine(rng, &other.suicide_weights),
        }
    }
}

impl ParentCombination for WeightsTree {
    fn parent_combine(&self, rng: &mut Rng, other: &Self) -> Self {
        choose_random(rng, self, other)
    }
}

impl ParentCombination for PlantCellAbilities {
    fn parent_combine(&self, rng: &mut Rng, other: &Self) -> Self {
        Self {
            sunlight_consumption: choose_random(
                rng,
                &self.sunlight_consumption,
                &other.sunlight_consumption,
            ),
            air_consumption: choose_random(rng, &self.air_consumption, &other.air_consumption),
            minerals_consumption: choose_random(
                rng,
                &self.minerals_consumption,
                &other.minerals_consumption,
            ),
            water_consumption: choose_random(
                rng,
                &self.water_consumption,
                &other.water_consumption,
            ),
            energy_production_speed: choose_random(
                rng,
                &self.energy_production_speed,
                &other.energy_production_speed,
            ),
            seed: choose_random(rng, &self.seed, &other.seed),
            grow_cost: 0.,
            passive_cost: 0.,
        }
        .with_populated_cost()
    }
}

pub fn parent_combine(
    rng: &mut Rng,
    data: &[PlantEvolutionData],
    children: usize,
) -> Vec<PlantEvolutionData> {
    (0..children)
        .map(|_| {
            let idx1 = rng.random_range(0..data.len());
            let idx2 = {
                let idx = rng.random_range(0..data.len() - 1);
                if idx >= idx1 { idx + 1 } else { idx }
            };
            data[idx1].parent_combine(rng, &data[idx2])
        })
        .collect()
}
