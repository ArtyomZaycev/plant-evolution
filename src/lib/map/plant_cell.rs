use std::cell::LazyCell;

use serde::{Deserialize, Serialize};

use crate::{
    evolution::{WithVolatility, consts::*},
    precalc::NUMBER_OF_CELLS,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlantCellAbilities {
    pub sunlight_consumption: WithVolatility<f32>,
    pub air_consumption: WithVolatility<f32>,
    pub minerals_consumption: WithVolatility<f32>,
    pub water_consumption: WithVolatility<f32>,
    pub energy_production_speed: WithVolatility<f32>,
    pub seed: bool,

    pub grow_cost: f32,
    pub passive_cost: f32,
}

impl PlantCellAbilities {
    pub const DEFAULT_BASIC: LazyCell<Self> = LazyCell::new(|| {
        Self {
            sunlight_consumption: WithVolatility::new(0.1),
            air_consumption: WithVolatility::new(0.1),
            minerals_consumption: WithVolatility::new(0.1),
            water_consumption: WithVolatility::new(0.1),
            energy_production_speed: WithVolatility::new(0.1),
            seed: false,
            grow_cost: 0.,
            passive_cost: 0.,
        }
        .with_populated_cost()
    });

    pub const DEFAULT_SEED: LazyCell<Self> = LazyCell::new(|| {
        Self {
            sunlight_consumption: WithVolatility::new(0.),
            air_consumption: WithVolatility::new(0.),
            minerals_consumption: WithVolatility::new(1.),
            water_consumption: WithVolatility::new(1.),
            energy_production_speed: WithVolatility::new(0.4),
            seed: false,
            grow_cost: 0.,
            passive_cost: 0.,
        }
        .with_populated_cost()
    });

    pub fn populate_cost(&mut self) {
        self.grow_cost = (1.
            + *self.sunlight_consumption
            + *self.air_consumption
            + *self.minerals_consumption
            + *self.water_consumption
            + self.energy_production_speed.sqrt() * ENERGY_PRODUCTION_COST_MULTIPLIER)
            .powi(2)
            + if self.seed { SEED_COST } else { 0. };

        self.passive_cost = (*self.sunlight_consumption
            + *self.air_consumption
            + *self.minerals_consumption
            + *self.water_consumption
            + *self.energy_production_speed)
            * PASSIVE_COST_MULTIPLIER;
    }

    pub fn with_populated_cost(mut self) -> Self {
        self.populate_cost();
        self
    }
}

#[derive(Debug, Default, Clone)]
pub struct PlantCellInput {
    pub sunlight: f32,
    pub air: f32,
    pub minerals: f32,
    pub water: f32,
    pub cells_proximity_data: [[bool; NUMBER_OF_CELLS]; 4],
}

#[derive(Debug, Clone)]
pub struct PlantCell {
    pub t: usize,
    pub input: PlantCellInput,
}

impl PlantCell {
    pub fn is_none(&self) -> bool {
        self.t == usize::MAX
    }
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }
}

impl Default for PlantCell {
    fn default() -> Self {
        Self {
            t: usize::MAX,
            input: Default::default(),
        }
    }
}
