use std::sync::mpsc;

use rand::RngExt;

use crate::{cell::*, evolution::*, map::*};

pub type Rng = rand::rngs::ThreadRng;

fn apply_change_chance<F: FnOnce()>(change_chance: f32, random: f32, f: F) {
    if random < change_chance {
        f();
    }
}

fn randomize_value(value: &mut f32, random: f32, entropy: f32) {
    // if entropy = 1, value can be changed from MIN to MAX
    *value = *value + (random - 0.5) * entropy;
}

fn randomize_value_change_chance(
    value: &mut f32,
    rng: &mut Rng,
    change_chance: f32,
    change_entropy: f32,
    min: f32,
    max: f32,
) {
    apply_change_chance(change_chance, rng.random(), || {
        randomize_value(value, rng.random(), change_entropy);
        *value = value.clamp(min, max);
    });
}

fn randomize_bool_value_change_chance(
    value: &mut bool,
    rng: &mut Rng,
    change_chance: f32,
    change_entropy: f32,
) {
    apply_change_chance(change_chance, rng.random(), || {
        if rng.random::<f32>() > 0.5 {
            *value = !*value
        }
    });
}

pub trait RandomEvolution {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32);
}

impl<T: RandomEvolution> RandomEvolution for Vec<T> {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) {
        self.iter_mut().for_each(|v| {
            v.evolve_random(rng, change_chance, change_entropy);
        });
    }
}

impl<T: RandomEvolution, const N: usize> RandomEvolution for [T; N] {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) {
        self.iter_mut().for_each(|v| {
            v.evolve_random(rng, change_chance, change_entropy);
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
        self.suicide_weights
            .evolve_random(rng, change_chance, change_entropy);
    }
}

impl RandomEvolution for PlantCellAbilities {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) {
        randomize_value_change_chance(
            &mut self.sunlight_consumption,
            rng,
            change_chance,
            change_entropy,
            0.,
            1.,
        );
        randomize_value_change_chance(
            &mut self.air_consumption,
            rng,
            change_chance,
            change_entropy,
            0.,
            1.,
        );
        randomize_value_change_chance(
            &mut self.minerals_consumption,
            rng,
            change_chance,
            change_entropy,
            0.,
            1.,
        );
        randomize_value_change_chance(
            &mut self.water_consumption,
            rng,
            change_chance,
            change_entropy,
            0.,
            1.,
        );
        randomize_value_change_chance(
            &mut self.power_production_speed,
            rng,
            change_chance,
            change_entropy,
            0.,
            1.,
        );
        randomize_bool_value_change_chance(&mut self.seed, rng, change_chance, change_entropy);
        self.populate_cost();
    }
}

impl RandomEvolution for CellEvolutionWeights {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) {
        randomize_value_change_chance(
            &mut self.sunlight,
            rng,
            change_chance,
            change_entropy,
            -1.,
            1.,
        );
        randomize_value_change_chance(&mut self.air, rng, change_chance, change_entropy, -1., 1.);
        randomize_value_change_chance(
            &mut self.minerals,
            rng,
            change_chance,
            change_entropy,
            -1.,
            1.,
        );
        randomize_value_change_chance(&mut self.water, rng, change_chance, change_entropy, -1., 1.);
        for row in &mut self.cells_proximity_data {
            for v in row {
                randomize_value_change_chance(v, rng, change_chance, change_entropy, -1., 1.);
            }
        }
        randomize_value_change_chance(
            &mut self.height,
            rng,
            change_chance,
            change_entropy,
            -1.,
            1.,
        );
        randomize_value_change_chance(&mut self.xdist, rng, change_chance, change_entropy, -1., 1.);
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
