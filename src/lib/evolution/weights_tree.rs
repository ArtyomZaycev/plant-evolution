use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::{map::PlantCellInput, precalc::NUMBER_OF_CELLS, utils::formula::{self, Formula, FormulaNode}};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InputNode {
    Sunlight,
    Air,
    Minerals,
    Water,
    Proximity { dir: usize, ctype: usize },
    Height,
    XDist,
}

impl formula::ParameterId for InputNode {
    fn get_name(&self) -> String {
        match self {
            InputNode::Sunlight => "sunlight".to_owned(),
            InputNode::Air => "air".to_owned(),
            InputNode::Minerals => "minerals".to_owned(),
            InputNode::Water => "water".to_owned(),
            InputNode::Proximity { dir, ctype } => format!("proximity[{dir}][{ctype}]"),
            InputNode::Height => "height".to_owned(),
            InputNode::XDist => "xdist".to_owned(),
        }
    }
}

pub type WeightsTreeParameters = (PlantCellInput, f32, f32);

impl formula::Parameters for WeightsTreeParameters {
    type ParameterId = InputNode;

    fn get_value(&self, id: &Self::ParameterId) -> f32 {
        let (input, height, xdist) = self;
        match id {
            InputNode::Sunlight => input.sunlight,
            InputNode::Air => input.air,
            InputNode::Minerals => input.minerals,
            InputNode::Water => input.water,
            InputNode::Proximity { dir, ctype } => {
                if input.cells_proximity_data[*dir][*ctype] {
                    1.
                } else {
                    0.
                }
            }
            InputNode::Height => *height,
            InputNode::XDist => *xdist,
        }
    }
}

pub type WeightsTree = Formula<WeightsTreeParameters>;

impl WeightsTree {
    pub fn calculate_safe(&self, input: &PlantCellInput, height: f32, xdist: f32) -> f32 {
        let value = self.calculate(&(input.clone(), height, xdist));
        if !value.is_normal() { 0. } else { value }
    }
}

impl Default for WeightsTree {
    fn default() -> Self {
        Self {
            nodes: vec![FormulaNode::Value(Default::default())],
        }
    }
}

impl rand::distr::Distribution<InputNode> for rand::distr::StandardUniform {
    fn sample<R: rand::prelude::Rng + ?Sized>(&self, rng: &mut R) -> InputNode {
        match rng.random_range(0..7) {
            0 => InputNode::Sunlight,
            1 => InputNode::Air,
            2 => InputNode::Minerals,
            3 => InputNode::Water,
            4 => InputNode::Proximity { dir: rng.random_range(0..4), ctype: rng.random_range(0..NUMBER_OF_CELLS) },
            5 => InputNode::Height,
            6 => InputNode::XDist,
            _ => panic!(),
        }
    }
}