use std::sync::mpsc;

use rand::RngExt;

use crate::{cell::*, evolution::*, map::*};

pub type Rng = rand::rngs::ThreadRng;

fn apply_change_chance<F: FnOnce()>(change_chance: f32, random: f32, f: F) {
    if random < change_chance {
        f();
    }
}

fn randomize_value(value: &mut f32, random: f32, entropy: f32, min: f32, max: f32) {
    // if entropy = 1, value can be changed from MIN to MAX
    *value = (*value + (random - 0.5) * 2. * entropy).clamp(min, max);
}

fn randomize_value_change_chance(
    value: &mut f32,
    rng: &mut Rng,
    change_chance: f32,
    change_entropy: f32,
) {
    apply_change_chance(change_chance, rng.random(), || {
        randomize_value(value, rng.random(), change_entropy, 0., 1.)
    });
}

fn randomize_value_change_chance_clamp(
    value: &mut f32,
    rng: &mut Rng,
    change_chance: f32,
    change_entropy: f32,
    min: f32, max:f32,
) {
    apply_change_chance(change_chance, rng.random(), || {
        randomize_value(value, rng.random(), change_entropy, -1., 1.);
        *value = value.clamp(min, max);
    });
}

pub trait RandomEvolution {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32);
}

impl<T: RandomEvolution> RandomEvolution for Vec<T> {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) {
        self.iter_mut().for_each(|v| {
            apply_change_chance(change_chance, rng.random(), || {
                v.evolve_random(rng, change_chance, change_entropy)
            });
        });
    }
}

impl<T: RandomEvolution, const N: usize> RandomEvolution for [T; N] {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) {
        self.iter_mut().for_each(|v| {
            apply_change_chance(change_chance, rng.random(), || {
                v.evolve_random(rng, change_chance, change_entropy)
            });
        });
    }
}

impl RandomEvolution for PlantEvolutionData {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) {
        self.cells_evolution_data
            .evolve_random(rng, change_chance, change_entropy);
        self.cells_abilities
            .evolve_random(rng, change_chance, change_entropy);
    }
}

impl RandomEvolution for CellEvolutionData {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) {
        self.weights
            .evolve_random(rng, change_chance, change_entropy);
    }
}

impl RandomEvolution for PlantCellAbilities {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) {
        randomize_value_change_chance_clamp(
            &mut self.sunlight_consumption,
            rng,
            change_chance,
            change_entropy,
            0., 1.
        );
        randomize_value_change_chance_clamp(
            &mut self.air_consumption,
            rng,
            change_chance,
            change_entropy,
            0., 1.
        );
        randomize_value_change_chance_clamp(
            &mut self.minerals_consumption,
            rng,
            change_chance,
            change_entropy,
            0., 1.
        );
        randomize_value_change_chance_clamp(
            &mut self.water_consumption,
            rng,
            change_chance,
            change_entropy,
            0., 1.
        );
        randomize_value_change_chance_clamp(
            &mut self.power_production_speed,
            rng,
            change_chance,
            change_entropy,
            0., 1.
        );
        self.populate_cost();
    }
}

impl RandomEvolution for CellEvolutionWeights {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) {
        randomize_value_change_chance(
            &mut self.weights.sunlight,
            rng,
            change_chance,
            change_entropy,
        );
        randomize_value_change_chance(&mut self.weights.air, rng, change_chance, change_entropy);
        randomize_value_change_chance(
            &mut self.weights.minerals,
            rng,
            change_chance,
            change_entropy,
        );
        randomize_value_change_chance(&mut self.weights.water, rng, change_chance, change_entropy);
        for row in &mut self.weights.cells_proximity_data {
            for v in row {
                randomize_value_change_chance(v, rng, change_chance, change_entropy);
            }
        }
    }
}

impl RandomEvolution for PlantCellProximityData {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) {
        randomize_value_change_chance(&mut self.direction, rng, change_chance, change_entropy);
        randomize_value_change_chance(&mut self.distance, rng, change_chance, change_entropy);
    }
}

pub fn run_evolution_random(
    sender: Option<mpsc::Sender<RunningEvolutionData>>,
    maps: &mut Vec<MapData>,
    evolutions: usize,
    evolve_steps: usize,
    change_chance: f32,
    change_entropy: f32,
) {
    let mut rng = rand::rng();
    run_evolution(
        sender,
        maps,
        |maps| {
            maps.evolve_random(&mut rng, change_chance, change_entropy);
        },
        evolutions,
        evolve_steps,
    );
}
