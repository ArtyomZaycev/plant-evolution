use std::sync::mpsc;

use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::{
    cell::*, const_precalc::*, map::*, parents_evolution::parent_combine,
    random_evolution::RandomEvolution, weights_tree::*,
};

type Rng = rand::rngs::ThreadRng;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellEvolutionData {
    pub weights: [[WeightsTree; NUMBER_OF_CELLS]; 3],
    pub suicide_weights: WeightsTree,
}

impl WeightsTree {
    fn rand_generate(rng: &mut Rng) -> Self {
        Self {
            nodes: vec![TreeNode::Input(InputNode::generate(rng))],
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
            grow_cost: 0.,
            passive_cost: 0.,
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
            grow_cost: 0.,
            passive_cost: 0.,
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

#[hotpath::measure]
fn sample_best_maps_evolution(maps: &mut Vec<MapData>, samples: usize) -> Vec<PlantEvolutionData> {
    maps.sort_by(|a, b| {
        calculate_score(a)
            .partial_cmp(&calculate_score(b))
            .unwrap()
            .reverse()
    });

    maps.iter()
        .take(samples)
        .map(|map| map.evolution_data.clone())
        .collect::<Vec<_>>()
}

#[hotpath::measure]
pub fn random_evolve(
    rng: &mut Rng,
    maps: &mut Vec<MapData>,
    samples: usize,
    change_chance: f32,
    change_entropy: f32,
) {
    let best_evolution_data = sample_best_maps_evolution(maps, samples);
    best_evolution_data
        .iter()
        .enumerate()
        .for_each(|(i, data)| {
            maps[i].evolution_data = data.clone();
        });
    maps.iter_mut()
        .skip(samples)
        .enumerate()
        .for_each(|(i, map)| {
            map.evolution_data = best_evolution_data[i % samples].clone();
            map.evolve_random(rng, change_chance, change_entropy);
        });
    maps.iter_mut().for_each(|map| map.restart());
}

#[hotpath::measure]
pub fn parents_random_evolve(
    rng: &mut Rng,
    maps: &mut Vec<MapData>,
    samples: usize,
    change_chance: f32,
    change_entropy: f32,
) {
    let best_evolution_data = sample_best_maps_evolution(maps, samples);
    let children_evolution_data = parent_combine(rng, &best_evolution_data, maps.len() - samples);

    best_evolution_data
        .iter()
        .enumerate()
        .for_each(|(i, data)| {
            maps[i].evolution_data = data.clone();
        });
    children_evolution_data
        .iter()
        .enumerate()
        .for_each(|(i, data)| {
            maps[i + samples].evolution_data = data.clone();
            if rng.random_bool(0.75) {
                maps[i + samples].evolve_random(rng, change_chance, change_entropy);
            }
        });
    maps.iter_mut().for_each(|map| map.restart());
}
