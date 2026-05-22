use rand::RngExt;

use crate::{cell::*, const_precalc::*, map::*};

type Rng = rand::rngs::ThreadRng;

#[derive(Debug, Clone)]
pub struct CellEvolutionWeights {
    pub weights: PlantCellInput,
}

impl CellEvolutionWeights {
    fn rand_generate(rng: &mut Rng) -> Self {
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
    fn rand_generate(rng: &mut Rng) -> Self {
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
    pub cells_abilities: [PlantCellAbilities; NUMBER_OF_CELLS],
}

impl PlantEvolutionData {
    pub fn generate() -> Self {
        Self::rand_generate(&mut rand::rng())
    }

    fn rand_generate(rng: &mut Rng) -> Self {
        let basic_cell = PlantCellAbilities {
            sunlight_consumption: 0.1,
            air_consumption: 0.1,
            minerals_consumption: 0.1,
            water_consumption: 0.1,
            power_production_speed: 0.1,
            cost: 0.,
        }
        .with_populated_cost();

        let cells = [
            PlantCellAbilities {
                sunlight_consumption: 1.,
                air_consumption: 1.,
                minerals_consumption: 1.,
                water_consumption: 1.,
                power_production_speed: 1.,
                cost: 0.,
            }
            .with_populated_cost(),
            basic_cell.clone(),
            basic_cell.clone(),
            basic_cell.clone(),
            basic_cell.clone(),
            basic_cell.clone(),
            basic_cell.clone(),
            basic_cell.clone(),
        ];

        Self {
            cells_evolution_data: (0..NUMBER_OF_CELLS)
                .map(|_| CellEvolutionData::rand_generate(rng))
                .collect::<Vec<CellEvolutionData>>()
                .try_into()
                .unwrap(),
            cells_abilities: cells,
        }
    }
}

pub fn calculate_score(map: &MapData) -> f32 {
    map.map.iter().fold(0., |acc, row| {
        row.iter().fold(acc, |acc, cell| match cell {
            MapCell::Air => acc,
            MapCell::Soil(_) => acc,
            MapCell::Plant(plant_cell) => {
                acc + map.evolution_data.cells_abilities[plant_cell.t].cost
            }
        })
    })
}

pub fn sample_maps(maps: &mut Vec<MapData>) {
    maps.sort_by(|a, b| {
        calculate_score(a)
            .partial_cmp(&calculate_score(b))
            .unwrap()
            .reverse()
    });

    let sample_size = maps.len() / 10;
    let best_evolution_data = maps
        .iter()
        .take(sample_size)
        .map(|map| map.evolution_data.clone())
        .collect::<Vec<_>>();

    // always 11
    let samples_per_best = maps.len() / sample_size + 1;
    best_evolution_data.iter().enumerate().for_each(|(i, evolution_data)| {
        maps.iter_mut().skip(samples_per_best * i).take(samples_per_best).for_each(|map| {
            map.evolution_data = evolution_data.clone();
            map.restart();
        });
    });
}

pub fn run_evolution<F: FnMut(&mut Vec<MapData>)>(maps: &mut Vec<MapData>, mut evolve: F, evolutions: usize, evolve_steps: usize) {
    (0..evolutions).for_each(|_| {
        (0..evolve_steps).for_each(|_| {
            maps.iter_mut().for_each(|map| map.tick());
        });
        evolve(maps);
    });
}