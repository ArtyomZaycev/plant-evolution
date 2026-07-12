use std::sync::mpsc;

use rand::RngExt;
use serde::{Deserialize, Serialize};

use super::{evolution_volatility::*, parents_evolution::*, random_evolution::*, weights_tree::*};
use crate::{map::*, precalc::*, utils::Rng};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellEvolutionData {
    pub weights: [[WithVolatility<WeightsTree>; NUMBER_OF_CELLS]; 3],
    pub suicide_weights: WithVolatility<WeightsTree>,
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
            weights: std::array::from_fn(|_| {
                std::array::from_fn(|_| WithVolatility::new(WeightsTree::rand_generate(rng)))
            }),
            suicide_weights: WithVolatility::new(WeightsTree {
                nodes: vec![TreeNode::Value(0.)],
            }),
        }
    }

    pub fn calc_suicide(&self, input: &PlantCellInput, height: f32, xdist: f32) -> f32 {
        self.suicide_weights.calculate(input, height, xdist)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantEvolutionData {
    pub evolutions: u32,
    pub cells_evolution_data: [WithVolatility<CellEvolutionData>; NUMBER_OF_CELLS],
    pub cells_abilities: [PlantCellAbilities; NUMBER_OF_CELLS],
}

impl PlantEvolutionData {
    pub fn generate(rng: &mut Rng) -> Self {
        Self::rand_generate(rng)
    }

    fn rand_generate(rng: &mut Rng) -> Self {
        let basic_cell = PlantCellAbilities {
            sunlight_consumption: WithVolatility::new(0.1),
            air_consumption: WithVolatility::new(0.1),
            minerals_consumption: WithVolatility::new(0.1),
            water_consumption: WithVolatility::new(0.1),
            energy_production_speed: WithVolatility::new(0.1),
            seed: false,
            grow_cost: 0.,
            passive_cost: 0.,
        }
        .with_populated_cost();

        let mut cells = std::array::repeat(basic_cell);
        cells[0] = PlantCellAbilities {
            sunlight_consumption: WithVolatility::new(0.),
            air_consumption: WithVolatility::new(0.),
            minerals_consumption: WithVolatility::new(1.),
            water_consumption: WithVolatility::new(1.),
            energy_production_speed: WithVolatility::new(0.4),
            seed: false,
            grow_cost: 0.,
            passive_cost: 0.,
        }
        .with_populated_cost();

        Self {
            evolutions: 0,
            cells_evolution_data: std::array::from_fn(|_| {
                WithVolatility::new(CellEvolutionData::rand_generate(rng))
            }),
            cells_abilities: cells,
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

#[hotpath::measure]
fn sample_best_maps_evolution(maps: &mut Vec<MapData>, samples: usize) -> Vec<PlantEvolutionData> {
    let mut best_maps_idx = maps
        .iter()
        .enumerate()
        .map(|(i, map)| (map.calculate_score(), i))
        .collect::<Vec<_>>();
    best_maps_idx.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().reverse());

    best_maps_idx
        .iter()
        .take(samples)
        .map(|(_, i)| maps[*i].evolution_data.clone())
        .collect::<Vec<_>>()
}

#[hotpath::measure]
pub fn random_evolve(
    rng: &mut Rng,
    maps: &mut Vec<MapData>,
    plants: usize,
    samples: usize,
    change_chance: f32,
    change_entropy: f32,
) {
    let best_evolution_data = sample_best_maps_evolution(maps, samples);
    maps.resize(plants, MapData::default());
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
    plants: usize,
    samples: usize,
    change_chance: f32,
    change_entropy: f32,
) {
    let best_evolution_data = sample_best_maps_evolution(maps, samples);
    maps.resize(plants, MapData::default());
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
