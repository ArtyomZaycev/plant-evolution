use std::sync::mpsc;

use serde::{Deserialize, Serialize};

use crate::{cell::*, const_precalc::*, map::*, weights_tree::*};

type Rng = rand::rngs::ThreadRng;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellEvolutionData {
    pub weights: [[WeightsTree; NUMBER_OF_CELLS]; 3],
    pub suicide_weights: WeightsTree,
}

impl WeightsTree {
    fn rand_generate(rng: &mut Rng) -> Self {
        Self {
            nodes: vec![TreeNode::Input(InputNode::generate(rng))]
        }
    }
}

impl CellEvolutionData {
    fn rand_generate(rng: &mut Rng) -> Self {
        Self {
            weights: (0..3)
                .map(|_| {
                    (0..NUMBER_OF_CELLS)
                        .map(|_| WeightsTree::rand_generate(rng))
                        .collect::<Vec<WeightsTree>>()
                        .try_into()
                        .unwrap()
                })
                .collect::<Vec<[WeightsTree; NUMBER_OF_CELLS]>>()
                .try_into()
                .unwrap(),
            suicide_weights: WeightsTree {
                nodes: vec![TreeNode::Value(0.)],
            },
        }
    }

    pub fn calc_suicide(&self, input: &PlantCellInput, height: f32, xdist: f32) -> f32 {
        self.suicide_weights.calculate(input, height, xdist)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            energy_production_speed: 0.1,
            seed: false,
            cost: 0.,
        }
        .with_populated_cost();

        let mut cells = std::array::repeat(basic_cell);
        cells[0] = PlantCellAbilities {
            sunlight_consumption: 0.,
            air_consumption: 0.,
            minerals_consumption: 1.,
            water_consumption: 1.,
            energy_production_speed: 0.4,
            seed: false,
            cost: 0.,
        }
        .with_populated_cost();

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
    let mut seeds = vec![];

    let nutrition = map
        .plants_pos
        .iter()
        .fold(PlantNutrition::default(), |nutrition, &(j, i)| {
            let cell = &map.plants[i][j];
            let cell_abilities = &map.evolution_data.cells_abilities[cell.t];
            if cell_abilities.seed && matches!(map.map[i][j], MapCell::Air(_)) {
                seeds.push((j, i));
            }
            PlantNutrition {
                sunlight: nutrition.sunlight
                    + cell.input.sunlight * cell_abilities.sunlight_consumption,
                air: nutrition.air + cell.input.air * cell_abilities.air_consumption,
                minerals: nutrition.minerals
                    + cell.input.minerals * cell_abilities.minerals_consumption,
                water: nutrition.water + cell.input.water * cell_abilities.water_consumption,
                energy: nutrition.energy + cell_abilities.energy_production_speed,
            }
        });

    let mut seeds_score: f32 = 0.;
    for &(x, y) in &seeds {
        let mut cnt = 0;
        for &(x2, y2) in &seeds {
            if x != x2 || y != y2 {
                if (x as f32 - x2 as f32).powi(2) + (y as f32 - y2 as f32).powi(2) < 25. {
                    cnt += 1;
                }
            }
        }
        seeds_score += 2. / (cnt + 1) as f32;
    }

    (seeds_score * 10.)
        + ([
            nutrition.sunlight,
            nutrition.air,
            nutrition.minerals,
            nutrition.water,
            nutrition.energy,
        ]
        .into_iter()
        .reduce(f32::min)
        .unwrap()
            * 100.)
            .sqrt()
}

#[hotpath::measure]
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

#[hotpath::measure]
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
