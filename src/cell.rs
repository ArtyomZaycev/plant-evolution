use rand::{RngExt, rngs::ThreadRng};

pub const NUMBER_OF_CELLS: usize = 8;

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
    pub fn populate_cost(self) -> Self {
        Self {
            cost: 1.
                + (self.sunlight_consumption
                    + self.air_consumption
                    + self.minerals_consumption
                    + self.water_consumption
                    + self.power_production_speed * 2.)
                    .powi(2),
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlantCellProximityData {
    pub distance: f32,
    pub direction: f32,
}

impl Default for PlantCellProximityData {
    fn default() -> Self {
        Self {
            distance: 1.,
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
    pub cells_proximity_data: [PlantCellProximityData; NUMBER_OF_CELLS],
}

#[derive(Debug, Clone)]
pub struct CellEvolutionWeights {
    weights: PlantCellInput,
}

impl CellEvolutionWeights {
    fn rand_generate(rng: &mut ThreadRng) -> Self {
        Self {
            weights: PlantCellInput {
                sunlight: rng.random(),
                air: rng.random(),
                minerals: rng.random(),
                water: rng.random(),
                cells_proximity_data: (0..NUMBER_OF_CELLS)
                    .map(|_| PlantCellProximityData {
                        distance: rng.random(),
                        direction: rng.random(),
                    })
                    .collect::<Vec<PlantCellProximityData>>()
                    .try_into()
                    .unwrap(),
            },
        }
    }
}

impl CellEvolutionWeights {
    pub fn calc_cell(&self, input: &PlantCellInput) -> f32 {
        input.sunlight * self.weights.sunlight
            + input.air * self.weights.air
            + input.minerals * self.weights.minerals
            + input.water * self.weights.water
            + input
                .cells_proximity_data
                .iter()
                .enumerate()
                .map(|(i, input)| input.direction * self.weights.cells_proximity_data[i].direction)
                .sum::<f32>()
            + input
                .cells_proximity_data
                .iter()
                .enumerate()
                .map(|(i, input)| input.distance * self.weights.cells_proximity_data[i].distance)
                .sum::<f32>()
    }
}

#[derive(Debug, Clone)]
pub struct CellEvolutionData {
    pub weights: [[CellEvolutionWeights; NUMBER_OF_CELLS]; 3],
}

impl CellEvolutionData {
    fn rand_generate(rng: &mut ThreadRng) -> Self {
        Self {
            weights: (0..3)
                .map(|_| {
                    (0..NUMBER_OF_CELLS)
                        .map(|_| CellEvolutionWeights::rand_generate(rng))
                        .collect::<Vec<CellEvolutionWeights>>()
                        .try_into()
                        .unwrap()
                })
                .collect::<Vec<[CellEvolutionWeights; NUMBER_OF_CELLS]>>()
                .try_into()
                .unwrap(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlantEvolutionData {
    pub cells_evolution_data: [CellEvolutionData; NUMBER_OF_CELLS],
}

impl PlantEvolutionData {
    pub fn generate() -> Self {
        Self::rand_generate(&mut rand::rng())
    }

    fn rand_generate(rng: &mut ThreadRng) -> Self {
        Self {
            cells_evolution_data: (0..NUMBER_OF_CELLS)
                .map(|_| CellEvolutionData::rand_generate(rng))
                .collect::<Vec<CellEvolutionData>>()
                .try_into()
                .unwrap(),
        }
    }
}
