use std::sync::mpsc;

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
                .map(|(i, input)| {
                    if input.is_some() {
                        input.direction * self.weights.cells_proximity_data[i].direction
                        + input.distance * self.weights.cells_proximity_data[i].distance
                    } else {
                        0.
                    }
                })
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
                power_production_speed: 0.2,
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
    let nutrition =
        map.plants_pos
            .iter()
            .fold(PlantNutrition::default(), |nutrition, &(j, i)| {
                let cell = &map.plants[i][j];
                PlantNutrition {
                    sunlight: nutrition.sunlight
                        + cell.input.sunlight
                            * map.evolution_data.cells_abilities[cell.t].sunlight_consumption,
                    air: nutrition.air
                        + cell.input.air
                            * map.evolution_data.cells_abilities[cell.t].air_consumption,
                    minerals: nutrition.minerals
                        + cell.input.minerals
                            * map.evolution_data.cells_abilities[cell.t].minerals_consumption,
                    water: nutrition.water
                        + cell.input.water
                            * map.evolution_data.cells_abilities[cell.t].water_consumption,
                    power: 0.,
                }
            });
            
    let score = [
        nutrition.sunlight,
        nutrition.air,
        nutrition.minerals,
        nutrition.water,
    ]
    .into_iter()
    .reduce(f32::min)
    .unwrap();

    score
    
/*
    let mut score = 0.;
    for &(j, i) in &map.plants_pos {
        score += map.evolution_data.cells_abilities[map.plants[i][j].t].cost;
    }
    score - map.evolution_data.cells_abilities[0].cost */
}

fn sample_maps(maps: &mut Vec<MapData>, samples: usize) {
    maps.sort_by(|a, b| {
        calculate_score(a)
            .partial_cmp(&calculate_score(b))
            .unwrap()
            .reverse()
    });

    let sample_size = maps.len() / samples;
    let best_evolution_data = maps
        .iter()
        .take(sample_size)
        .map(|map| map.evolution_data.clone())
        .collect::<Vec<_>>();

    best_evolution_data
        .iter()
        .enumerate()
        .for_each(|(i, evolution_data)| {
            maps.iter_mut()
                .skip(samples * i)
                .take(samples)
                .for_each(|map: &mut MapData| {
                    map.evolution_data = evolution_data.clone();
                    map.restart();
                });
        });
}

pub fn sample_evolve_maps<F: FnMut(&mut MapData)>(
    maps: &mut Vec<MapData>,
    samples: usize,
    mut evolve: F,
) {
    sample_maps(maps, samples);
    for (i, map) in maps.iter_mut().enumerate() {
        if i % samples != 0 {
            evolve(map)
        }
    }
}

#[derive(Debug)]
pub struct RunningEvolutionData {
    pub evolution_total: usize,
    pub tick_total: usize,

    pub evolution: usize,
    pub tick: usize,
}

pub fn run_evolution<F: FnMut(&mut Vec<MapData>)>(
    sender: Option<mpsc::Sender<RunningEvolutionData>>,
    maps: &mut Vec<MapData>,
    mut evolve: F,
    evolutions: usize,
    evolve_steps: usize,
) {
    (0..evolutions).for_each(|evolution: usize| {
        (0..evolve_steps).for_each(|tick| {
            maps.iter_mut().for_each(|map| map.tick());
            if tick % 100 == 0 {
                let data = RunningEvolutionData {
                    evolution_total: evolutions,
                    tick_total: evolve_steps,
                    evolution,
                    tick,
                };
                println!("Data: {data:?}");
                if let Some(sender) = &sender {
                    let _ = sender.send(data);
                }
            }
        });
        evolve(maps);
    });
}
