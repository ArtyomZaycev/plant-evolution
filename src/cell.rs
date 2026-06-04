use serde::{Deserialize, Serialize};

use crate::const_precalc::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantCellAbilities {
    pub sunlight_consumption: f32,
    pub air_consumption: f32,
    pub minerals_consumption: f32,
    pub water_consumption: f32,
    pub power_production_speed: f32,
    pub seed: bool,

    pub cost: f32,
}

impl PlantCellAbilities {
    pub fn populate_cost(&mut self) {
        self.cost = (1.
            + self.sunlight_consumption
            + self.air_consumption
            + self.minerals_consumption
            + self.water_consumption
            + self.power_production_speed.sqrt() * 4.)
            .powi(3)
            / 10.
            + if self.seed { 50. } else { 0. };
    }

    pub fn with_populated_cost(self) -> Self {
        let mut s = self;
        s.populate_cost();
        s
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
