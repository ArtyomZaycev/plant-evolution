use std::cell::LazyCell;

use crate::{evolution::WithVolatility, map::PlantCellAbilities};

pub const MIN_VOLATILITY: f32 = 0.1;
pub const MAX_VOLATILITY: f32 = 2.;

pub const VOLATILITY_PMULTIPLIER: f32 = 1.2;
pub const VOLATILITY_NMULTIPLIER: f32 = 0.992;

pub const MIN_AFTER_VOLATILITY: f32 = 0.05;
pub const MAX_AFTER_VOLATILITY: f32 = 0.9;

pub const DEFAULT_BASIC_CELL: LazyCell<PlantCellAbilities> = LazyCell::new(|| PlantCellAbilities {
    sunlight_consumption: WithVolatility::new(0.1),
    air_consumption: WithVolatility::new(0.1),
    minerals_consumption: WithVolatility::new(0.1),
    water_consumption: WithVolatility::new(0.1),
    energy_production_speed: WithVolatility::new(0.1),
    seed: false,
    grow_cost: 0.,
    passive_cost: 0.,
}.with_populated_cost());

pub const DEFAULT_SEED_CELL: LazyCell<PlantCellAbilities> = LazyCell::new(|| PlantCellAbilities {
    sunlight_consumption: WithVolatility::new(0.),
    air_consumption: WithVolatility::new(0.),
    minerals_consumption: WithVolatility::new(1.),
    water_consumption: WithVolatility::new(1.),
    energy_production_speed: WithVolatility::new(0.4),
    seed: false,
    grow_cost: 0.,
    passive_cost: 0.,
}
.with_populated_cost());


pub const DEFAULT_THREAD_COUNT: u32 = 8;

pub const DEFAULT_NUMBER_OF_PLANTS: usize = 200;
pub const DEFAULT_NUMBER_OF_SAMPLES: usize = 10;
pub const DEFAULT_CHANGE_CHANCE: f32 = 0.05;
pub const DEFAULT_CHANGE_ENTROPY: f32 = 0.8;

// ??
pub const PARENTS_EVOLUTION_EVOLVE_CHANCE: f64 = 0.75;

pub const MAX_WEIGHTS_TREE_SIZE: usize = 40;

pub const SUNLIGHT_AIR_MULTIPLIER: f32 = 0.98;
pub const SUNLIGHT_CELL_MULTIPLIER: f32 = 0.3;

pub const AIR_AIR_MULTIPLIER: f32 = 1.;
pub const AIR_CELL_MULTIPLIER: f32 = 0.125;

pub const LOW_DEPTH_MINERALS: f32 = 0.1;
pub const LOW_DEPTH_WATER: f32 = 0.2;
pub const HIGH_DEPTH_MINERALS: f32 = 0.3;
pub const HIGH_DEPTH_WATER: f32 = 0.01;

pub const SEEDS_MIN_DISTANCE: usize = 5;

pub const SEED_SCORE: f32 = 10.;
pub const SCORE_NUTRITION_MULTIPLIER: f32 = 100.;

pub const ENERGY_PRODUCTION_COST_MULTIPLIER: f32 = 4.;
pub const SEED_COST: f32 = 50.;

pub const PASSIVE_COST_MULTIPLIER: f32 = 1. / 80.;