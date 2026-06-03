use crate::const_precalc::*;

#[derive(Debug, Clone)]
pub struct PlantCellAbilities {
    pub sunlight_consumption: f32,
    pub air_consumption: f32,
    pub minerals_consumption: f32,
    pub water_consumption: f32,
    pub power_production_speed: f32,

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
            .powi(3) / 10.;
    }

    pub fn with_populated_cost(self) -> Self {
        let mut s = self;
        s.populate_cost();
        s
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlantCellProximityData {
    pub distance: f32,
    pub direction: f32,
}

impl PlantCellProximityData {
    pub fn is_none(&self) -> bool {
        self.distance.is_nan()
    }
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }
}

impl Default for PlantCellProximityData {
    fn default() -> Self {
        Self {
            distance: f32::NAN,
            direction: 0.5,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct PlantCellInput {
    pub sunlight: f32,
    pub air: f32,
    pub minerals: f32,
    pub water: f32,
    pub cells_proximity_data: [[f32; NUMBER_OF_CELLS]; 4],
}
