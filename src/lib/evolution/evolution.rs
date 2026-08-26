use std::sync::mpsc;

use formula::FormulaNode;
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use serde::{Deserialize, Serialize};

use super::{evolution_volatility::*, parents_evolution::*, random_evolution::*, weights_tree::*};
use crate::{
    evolution::consts::*,
    map::*,
    precalc::*,
    utils::Rng,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CellEvolutionData {
    pub weights: [[WithVolatility<WeightsTree>; NUMBER_OF_CELLS]; 4],
    pub suicide_weights: WithVolatility<WeightsTree>,
}

impl WeightsTree {
    fn rand_generate(rng: &mut Rng) -> Self {
        Self::new(vec![FormulaNode::Parameter(rng.random())])
    }
}

impl CellEvolutionData {
    fn rand_generate(rng: &mut Rng) -> Self {
        Self {
            weights: std::array::from_fn(|_| {
                std::array::from_fn(|_| WithVolatility::new(WeightsTree::rand_generate(rng)))
            }),
            suicide_weights: WithVolatility::new(WeightsTree::new(vec![FormulaNode::Value(0.)])),
        }
    }

    pub fn calc_suicide(&self, input: &PlantCellInput, height: f32, xdist: f32) -> f32 {
        self.suicide_weights.calculate_safe(input, height, xdist)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
        let mut cells = std::array::repeat(PlantCellAbilities::DEFAULT_BASIC.clone());
        cells[0] = PlantCellAbilities::DEFAULT_SEED.clone();

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
            maps.iter_mut().for_each(|map| map.tick(false));
            if tick % 100 == 0 {
                let data = RunningEvolutionData {
                    evolution_total: evolutions,
                    tick_total: evolve_steps,
                    evolution,
                    tick,
                };
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

    // Select the top `samples` scores with an O(n) partition instead of a
    // full O(n log n) sort.
    let count = best_maps_idx.len().min(samples);
    if count > 0 && count < best_maps_idx.len() {
        let _ = best_maps_idx
            .select_nth_unstable_by(count - 1, |a, b| b.0.partial_cmp(&a.0).unwrap());
    }

    best_maps_idx
        .iter()
        .take(count)
        .map(|(_, i)| maps[*i].evolution_data.clone())
        .collect::<Vec<_>>()
}

/// Executes per-map work, passing each map's index. `max_threads` bounds the
/// parallelism (`usize::MAX` to use all available threads); sequential pools
/// ignore it. The engine supplies its persistent worker pool so threads are
/// reused across calls instead of being spawned per call.
pub trait EvolutionPool {
    fn for_each_map_mut(
        &mut self,
        maps: &mut [MapData],
        max_threads: usize,
        f: impl Fn(usize, &mut MapData) + Sync,
    );
}

/// Sequential executor for builds without the `thread_evolution` feature.
#[derive(Debug, Default)]
pub struct SequentialPool;

impl EvolutionPool for SequentialPool {
    fn for_each_map_mut(
        &mut self,
        maps: &mut [MapData],
        _max_threads: usize,
        f: impl Fn(usize, &mut MapData) + Sync,
    ) {
        maps.iter_mut().enumerate().for_each(|(i, map)| f(i, map));
    }
}

#[cfg(feature = "thread_evolution")]
impl EvolutionPool for scoped_threadpool::Pool {
    fn for_each_map_mut(
        &mut self,
        maps: &mut [MapData],
        max_threads: usize,
        f: impl Fn(usize, &mut MapData) + Sync,
    ) {
        let threads = max_threads.min(self.thread_count() as usize).min(maps.len());
        if threads <= 1 {
            maps.iter_mut().enumerate().for_each(|(i, map)| f(i, map));
            return;
        }
        let chunk_size = maps.len().div_ceil(threads);
        self.scoped(|scope| {
            let f = &f;
            for (chunk_idx, chunk) in maps.chunks_mut(chunk_size).enumerate() {
                let base = chunk_idx * chunk_size;
                scope.execute(move || {
                    chunk
                        .iter_mut()
                        .enumerate()
                        .for_each(|(j, map)| f(base + j, map));
                });
            }
        });
    }
}

#[hotpath::measure]
pub fn random_evolve<P: EvolutionPool>(
    pool: &mut P,
    max_threads: usize,
    rng: &mut Rng,
    maps: &mut Vec<MapData>,
    plants: usize,
    samples: usize,
    change_chance: f32,
    change_entropy: f32,
) {
    let best_evolution_data = sample_best_maps_evolution(maps, samples);
    maps.resize_with(plants, MapData::default);

    // Assign each child its parent genome (cheap Arc clones), then mutate with
    // a per-map seeded RNG so the work can be split across threads.
    let seeds: Vec<u64> = (0..maps.len()).map(|_| rng.random::<u64>()).collect();
    pool.for_each_map_mut(&mut maps[samples..], max_threads, |i, map| {
        map.evolution_data = best_evolution_data[i % samples].clone();
        let mut map_rng = SmallRng::seed_from_u64(seeds[samples + i]);
        map.evolve_random(&mut map_rng, change_chance, change_entropy);
    });

    best_evolution_data
        .into_iter()
        .enumerate()
        .for_each(|(i, data)| {
            maps[i].evolution_data = data;
        });
    pool.for_each_map_mut(maps, max_threads, |_, map| map.restart());
}

#[hotpath::measure]
pub fn parents_random_evolve<P: EvolutionPool>(
    pool: &mut P,
    max_threads: usize,
    rng: &mut Rng,
    maps: &mut Vec<MapData>,
    plants: usize,
    samples: usize,
    change_chance: f32,
    change_entropy: f32,
) {
    let best_evolution_data = sample_best_maps_evolution(maps, samples);
    maps.resize_with(plants, MapData::default);
    parent_combine(rng, &best_evolution_data, &mut maps[samples..]);

    best_evolution_data
        .into_iter()
        .enumerate()
        .for_each(|(i, data)| {
            maps[i].evolution_data = data;
        });

    let seeds: Vec<u64> = (0..maps.len()).map(|_| rng.random::<u64>()).collect();
    pool.for_each_map_mut(&mut maps[samples..], max_threads, |i, map| {
        let mut map_rng = SmallRng::seed_from_u64(seeds[samples + i]);
        if map_rng.random_bool(PARENTS_EVOLUTION_EVOLVE_CHANCE) {
            hotpath::measure_block!("parents_do_evolve", {
                map.evolve_random(&mut map_rng, change_chance, change_entropy);
            })
        }
    });
    pool.for_each_map_mut(maps, max_threads, |_, map| map.restart());
}
