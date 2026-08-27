use std::sync::{Arc, mpsc};

use formula::{FormulaNode, OpNode};
use rand::RngExt;

use super::{
    CellEvolutionData, PlantEvolutionData, RunningEvolutionData, run_evolution, weights_tree::*,
};
use crate::{
    evolution::{WithVolatility, consts::MAX_WEIGHTS_TREE_SIZE},
    map::*,
    utils::*,
};

fn apply_change_chance_and<F: FnOnce() -> bool>(change_chance: f32, random: f32, f: F) -> bool {
    if random < change_chance { f() } else { false }
}

fn apply_change_chance<F: FnOnce()>(change_chance: f32, random: f32, f: F) -> bool {
    if random < change_chance {
        f();
        true
    } else {
        false
    }
}

fn randomize_value(value: &mut f32, random: f32, entropy: f32) {
    // if entropy = 1, value can be changed from MIN to MAX
    *value += (random - 0.5) * entropy;
}

fn randomize_value_change_chance_volatile(
    value: &mut WithVolatility<f32>,
    rng: &mut Rng,
    change_chance: f32,
    change_entropy: f32,
    min: f32,
    max: f32,
) -> bool {
    let old_value = value.value;
    let changed = value.evolve_random(rng, change_chance, change_entropy);
    if changed {
        value.value = value.clamp(min, max);
        old_value != value.value
    } else {
        false
    }
}

fn randomize_bool_value_change_chance(value: &mut bool, rng: &mut Rng, change_chance: f32) -> bool {
    apply_change_chance_and(change_chance, rng.random(), || {
        if rng.random::<f32>() > 0.5 {
            *value = !*value;
            true
        } else {
            false
        }
    })
}

pub trait RandomEvolution {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) -> bool;
}

impl<T: RandomEvolution> RandomEvolution for Vec<T> {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) -> bool {
        self.iter_mut().fold(false, |acc, v| {
            acc | v.evolve_random(rng, change_chance, change_entropy)
        })
    }
}

impl<T: RandomEvolution, const N: usize> RandomEvolution for [T; N] {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) -> bool {
        self.iter_mut().fold(false, |acc, v| {
            acc | v.evolve_random(rng, change_chance, change_entropy)
        })
    }
}

impl RandomEvolution for f32 {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) -> bool {
        apply_change_chance(change_chance, rng.random(), || {
            randomize_value(self, rng.random(), change_entropy);
        })
    }
}

impl RandomEvolution for PlantEvolutionData {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) -> bool {
        self.evolutions += 1;
        self.cells_evolution_data
            .evolve_random(rng, change_chance, change_entropy)
            | self
                .cells_abilities
                .evolve_random(rng, change_chance, change_entropy)
    }
}

impl RandomEvolution for CellEvolutionData {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) -> bool {
        apply_change_chance_and(change_chance, rng.random(), || {
            self.weights
                .evolve_random(rng, change_chance, change_entropy)
                | self
                    .suicide_weights
                    .evolve_random(rng, change_chance, change_entropy)
        })
    }
}

impl RandomEvolution for PlantCellAbilities {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) -> bool {
        let changed = randomize_value_change_chance_volatile(
            &mut self.sunlight_consumption,
            rng,
            change_chance,
            change_entropy,
            0.,
            1.,
        ) | randomize_value_change_chance_volatile(
            &mut self.air_consumption,
            rng,
            change_chance,
            change_entropy,
            0.,
            1.,
        ) | randomize_value_change_chance_volatile(
            &mut self.minerals_consumption,
            rng,
            change_chance,
            change_entropy,
            0.,
            1.,
        ) | randomize_value_change_chance_volatile(
            &mut self.water_consumption,
            rng,
            change_chance,
            change_entropy,
            0.,
            1.,
        ) | randomize_value_change_chance_volatile(
            &mut self.energy_production_speed,
            rng,
            change_chance,
            change_entropy,
            0.,
            1.,
        ) | randomize_bool_value_change_chance(&mut self.seed, rng, change_chance);
        if changed {
            self.populate_cost();
        }
        changed
    }
}

impl RandomEvolution for WeightsTree {
    fn evolve_random(&mut self, rng: &mut Rng, change_chance: f32, change_entropy: f32) -> bool {
        apply_change_chance(change_chance, rng.random(), || {
            let formula = Arc::make_mut(&mut self.formula);
            let nodes = &mut formula.nodes.nodes;
            let idx = rng.random_range(0..nodes.len());
            let allow_add = nodes.len() < MAX_WEIGHTS_TREE_SIZE;
            /*
                0 - tweak
                    Value - adjust Value
                    Input - change Input,
                    Operation - change within the same number of operands
                1 - replace
                    Replace Node for other random one
                2 - advance
                    Add operation where one of the operands is the initial node
            */
            let transform_type = rng.random_range(if allow_add { 0..=2 } else { 0..=1 });
            match transform_type {
                0 => match &mut nodes[idx] {
                    FormulaNode::Value(value) => {
                        if !value.is_normal() {
                            *value = 0.;
                        }
                        apply_change_chance(change_chance, rng.random(), || {
                            randomize_value(value, rng.random(), change_entropy);
                        });
                    }
                    FormulaNode::Parameter(input_node) => {
                        *input_node = rng.random();
                    }
                    FormulaNode::Operation(op_node) => match op_node {
                        OpNode::Unary(unary_op, _) => {
                            *unary_op = rng.random();
                        }
                        OpNode::Binary(binary_op, _, _) => {
                            *binary_op = rng.random();
                        }
                    },
                },
                1 => {
                    let (new_node, mut new_leaves) =
                        FormulaNode::generate(rng, nodes.len(), allow_add);
                    nodes[idx] = new_node;
                    nodes.append(&mut new_leaves);
                }
                2 => {
                    let (new_node, mut new_leaves) =
                        FormulaNode::generate_operation(rng, nodes.len());
                    if let FormulaNode::Operation(op_node) = new_node {
                        let op_node = match op_node {
                            OpNode::Unary(unary_op, idx1) => {
                                new_leaves = vec![nodes[idx]];
                                OpNode::Unary(unary_op, idx1)
                            }
                            OpNode::Binary(binary_op, idx1, idx2) => {
                                if rng.random_range(0..=1) == 0 {
                                    new_leaves = vec![nodes[idx], new_leaves[1]];
                                    OpNode::Binary(binary_op, idx1, idx2)
                                } else {
                                    new_leaves = vec![new_leaves[0], nodes[idx]];
                                    OpNode::Binary(binary_op, idx1, idx2)
                                }
                            }
                        };
                        nodes[idx] = FormulaNode::Operation(op_node);
                        nodes.append(&mut new_leaves);
                    }
                }
                _ => {
                    panic!("Unexpected transform_type");
                }
            }

            formula.nodes.compress();
        })
    }
}

pub fn run_evolution_random(
    sender: Option<mpsc::Sender<RunningEvolutionData>>,
    maps: &mut Vec<MapData>,
    rng: &mut Rng,
    evolutions: usize,
    evolve_steps: usize,
    change_chance: f32,
    change_entropy: f32,
) {
    run_evolution(
        sender,
        maps,
        |maps| {
            maps.evolve_random(rng, change_chance, change_entropy);
        },
        evolutions,
        evolve_steps,
    );
}
