use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::{cell::PlantCellInput, const_precalc::NUMBER_OF_CELLS};

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

impl InputNode {
    pub fn get_value(&self, input: &PlantCellInput, height: f32, xdist: f32) -> f32 {
        match &self {
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
            InputNode::Height => height,
            InputNode::XDist => xdist,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UnaryOp {
    Sqr,
    Sqrt,
    Ln,
    Inv,
    Minus,
}

impl UnaryOp {
    fn calc(&self, v1: f32) -> f32 {
        match &self {
            UnaryOp::Sqr => v1.powi(2),
            UnaryOp::Sqrt => v1.sqrt(),
            UnaryOp::Ln => {
                if v1 <= 0. {
                    0.
                } else {
                    v1.ln()
                }
            }
            UnaryOp::Inv => 1. / v1,
            UnaryOp::Minus => -v1,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl BinaryOp {
    fn calc(&self, v1: f32, v2: f32) -> f32 {
        match &self {
            BinaryOp::Add => v1 + v2,
            BinaryOp::Sub => v1 - v2,
            BinaryOp::Mul => v1 * v2,
            BinaryOp::Div => {
                if v2 == 0. {
                    0.
                } else {
                    v1 / v2
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OpNode {
    Unary(UnaryOp, usize),
    Binary(BinaryOp, usize, usize),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TreeNode {
    Value(f32),
    Input(InputNode),
    Operation(OpNode),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightsTree {
    pub nodes: Vec<TreeNode>,
}

impl WeightsTree {
    fn calc_inner(&self, input: &PlantCellInput, height: f32, xdist: f32, idx: usize) -> f32 {
        match &self.nodes[idx] {
            TreeNode::Value(value) => *value,
            TreeNode::Input(input_node) => input_node.get_value(input, height, xdist),
            TreeNode::Operation(op_node) => match op_node {
                OpNode::Unary(unary_op, idx1) => {
                    unary_op.calc(self.calc_inner(input, height, xdist, *idx1))
                }
                OpNode::Binary(binary_op, idx1, idx2) => binary_op.calc(
                    self.calc_inner(input, height, xdist, *idx1),
                    self.calc_inner(input, height, xdist, *idx2),
                ),
            },
        }
    }

    pub fn calculate(&self, input: &PlantCellInput, height: f32, xdist: f32) -> f32 {
        self.calc_inner(input, height, xdist, 0)
    }
}

impl WeightsTree {
    fn traverse_inner(&mut self, f: &mut Vec<bool>, idx: usize) {
        f[idx] = true;
        match self.nodes[idx] {
            TreeNode::Value(_) => {}
            TreeNode::Input(_) => {}
            TreeNode::Operation(op_node) => match op_node {
                OpNode::Unary(_, idx1) => {
                    self.traverse_inner(f, idx1);
                }
                OpNode::Binary(_, idx1, idx2) => {
                    self.traverse_inner(f, idx1);
                    self.traverse_inner(f, idx2);
                }
            },
        }
    }

    pub fn compact(&mut self) {
        let mut f = vec![false; self.nodes.len()];
        self.traverse_inner(&mut f, 0);
        let mut new_idx = vec![0; self.nodes.len()];
        f.iter().enumerate().fold(0, |cnt, (i, v)| {
            new_idx[i] = i - cnt;
            if *v {
                cnt
            } else {
                self.nodes.remove(i - cnt);
                cnt + 1
            }
        });
        self.nodes
            .iter_mut()
            .for_each(|node| match node {
                TreeNode::Operation(op_node) => match op_node {
                    OpNode::Unary(_, idx1) => {
                        *idx1 = new_idx[*idx1];
                    }
                    OpNode::Binary(_, idx1, idx2) => {
                        *idx1 = new_idx[*idx1];
                        *idx2 = new_idx[*idx2];
                    }
                },
                _ => {}
            });
    }

    fn get_subformula(&self, idx: usize) -> String {
        match &self.nodes[idx] {
            TreeNode::Value(value) => format!("{:.2}", value),
            TreeNode::Input(input_node) => match input_node {
                InputNode::Sunlight => "sunlight".to_owned(),
                InputNode::Air => "air".to_owned(),
                InputNode::Minerals => "minerals".to_owned(),
                InputNode::Water => "water".to_owned(),
                InputNode::Proximity { dir, ctype } => format!("proximity[{dir}][{ctype}]"),
                InputNode::Height => "height".to_owned(),
                InputNode::XDist => "xdist".to_owned(),
            },
            TreeNode::Operation(op_node) => match op_node {
                OpNode::Unary(unary_op, idx1) => match unary_op {
                    UnaryOp::Sqr => format!("{}^2", self.get_subformula(*idx1)),
                    UnaryOp::Sqrt => format!("sqrt({})", self.get_subformula(*idx1)),
                    UnaryOp::Ln => format!("ln({})", self.get_subformula(*idx1)),
                    UnaryOp::Inv => format!("{}^-1", self.get_subformula(*idx1)),
                    UnaryOp::Minus => format!("-{}", self.get_subformula(*idx1)),
                },
                OpNode::Binary(binary_op, idx1, idx2) => match binary_op {
                    BinaryOp::Add => format!(
                        "({} + {})",
                        self.get_subformula(*idx1),
                        self.get_subformula(*idx2)
                    ),
                    BinaryOp::Sub => format!(
                        "({} - {})",
                        self.get_subformula(*idx1),
                        self.get_subformula(*idx2)
                    ),
                    BinaryOp::Mul => format!(
                        "({} * {})",
                        self.get_subformula(*idx1),
                        self.get_subformula(*idx2)
                    ),
                    BinaryOp::Div => format!(
                        "({} / {})",
                        self.get_subformula(*idx1),
                        self.get_subformula(*idx2)
                    ),
                },
            },
        }
    }

    pub fn get_formula(&self) -> String {
        self.get_subformula(0)
    }
}

type Rng = rand::rngs::ThreadRng;

impl InputNode {
    pub fn generate(rng: &mut Rng) -> Self {
        match rng.random_range(0..=6) {
            0 => Self::Sunlight,
            1 => Self::Air,
            2 => Self::Minerals,
            3 => Self::Water,
            4 => Self::Proximity {
                dir: rng.random_range(0..4),
                ctype: rng.random_range(0..NUMBER_OF_CELLS),
            },
            5 => Self::Height,
            6 => Self::XDist,
            _ => {
                panic!("InputNode generate unexpected value");
            }
        }
    }
}

impl UnaryOp {
    pub fn generate(rng: &mut Rng) -> Self {
        match rng.random_range(0..=4) {
            0 => Self::Sqr,
            1 => Self::Sqrt,
            2 => Self::Ln,
            3 => Self::Inv,
            4 => Self::Minus,
            _ => {
                panic!("UnaryOp generate unexpected value");
            }
        }
    }
}

impl BinaryOp {
    pub fn generate(rng: &mut Rng) -> Self {
        match rng.random_range(0..=3) {
            0 => Self::Add,
            1 => Self::Sub,
            2 => Self::Mul,
            3 => Self::Div,
            _ => {
                panic!("BinaryOp generate unexpected value");
            }
        }
    }
}

impl TreeNode {
    pub fn generate(rng: &mut Rng, next_node_idx: usize, allow_op: bool) -> (Self, Vec<Self>) {
        match rng.random_range(if allow_op { 0..=2 } else { 0..=1 }) {
            0 => (Self::Value((rng.random::<f32>() - 0.5) * 2.), vec![]),
            1 => (Self::Input(InputNode::generate(rng)), vec![]),
            2 => match rng.random_range(0..=1) {
                0 => (
                    Self::Operation(OpNode::Unary(UnaryOp::generate(rng), next_node_idx)),
                    vec![Self::generate(rng, next_node_idx, false).0],
                ),
                1 => (
                    Self::Operation(OpNode::Binary(
                        BinaryOp::generate(rng),
                        next_node_idx,
                        next_node_idx + 1,
                    )),
                    vec![
                        Self::generate(rng, next_node_idx, false).0,
                        Self::generate(rng, next_node_idx, false).0,
                    ],
                ),
                _ => {
                    panic!("TreeNode OpNode generate unexpected value");
                }
            },
            _ => {
                panic!("TreeNode generate unexpected value");
            }
        }
    }
}
