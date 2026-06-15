use serde::{Deserialize, Serialize};

use crate::precalc::NUMBER_OF_CELLS;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantCellAbilities {
    pub sunlight_consumption: f32,
    pub air_consumption: f32,
    pub minerals_consumption: f32,
    pub water_consumption: f32,
    pub energy_production_speed: f32,
    pub seed: bool,

    pub grow_cost: f32,
    pub passive_cost: f32,
}

impl PlantCellAbilities {
    pub fn populate_cost(&mut self) {
        self.grow_cost = (1.
            + self.sunlight_consumption
            + self.air_consumption
            + self.minerals_consumption
            + self.water_consumption
            + self.energy_production_speed.sqrt() * 4.)
            .powi(2)
            + if self.seed { 50. } else { 0. };

        self.passive_cost = (self.sunlight_consumption
            + self.air_consumption
            + self.minerals_consumption
            + self.water_consumption
            + self.energy_production_speed)
            / 80.;
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
