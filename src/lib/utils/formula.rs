use serde::{Deserialize, Serialize};
use std::{fmt::Debug, ops::{Deref, DerefMut}};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UnaryOp {
    Sqr,
    Sqrt,
    Pow(f32),
    Powi(i32),
    Ln,
    Inv,
    Minus,
}

impl UnaryOp {
    fn calc(&self, v1: f32) -> f32 {
        match &self {
            UnaryOp::Sqr => v1.powi(2),
            UnaryOp::Sqrt => v1.sqrt(),
            UnaryOp::Pow(n) => v1.powf(*n),
            UnaryOp::Powi(n) => v1.powi(*n),
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
pub enum FormulaNode<PId: ParameterId> {
    Value(f32),
    Parameter(PId),
    Operation(OpNode),
}

pub trait ParameterId: Debug + Clone {
    fn get_name(&self) -> String;
}

pub trait Parameters<PId: ParameterId>: Debug {
    fn get_value(&self, id: &PId) -> f32;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formula<PId: ParameterId> {
    nodes: Vec<FormulaNode<PId>>,
    is_sorted: bool,
}

pub struct FormulaNodesGuard<'a, PId: ParameterId> {
    formula: &'a mut Formula<PId>,
}

impl<'a, PId: ParameterId> Deref for FormulaNodesGuard<'a, PId> {
    type Target = Vec<FormulaNode<PId>>;

    fn deref(&self) -> &Self::Target {
        &self.formula.nodes
    }
}

impl<'a, PId: ParameterId> DerefMut for FormulaNodesGuard<'a, PId> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.formula.is_sorted = false;
        &mut self.formula.nodes
    }
}

impl<'a, PId: ParameterId> Drop for FormulaNodesGuard<'a, PId> {
    fn drop(&mut self) {
        if !self.formula.is_sorted {
            self.formula.sort_nodes();
        }
    }
}

impl<PId: ParameterId> Formula<PId> {
    pub fn new(nodes: Vec<FormulaNode<PId>>) -> Self {
        assert!(!nodes.is_empty());
        let mut s = Self {
            nodes,
            is_sorted: false,
        };
        s.sort_nodes();
        s
    }
    
    pub fn get_nodes(&self) -> &Vec<FormulaNode<PId>> {
        &self.nodes
    }

    pub fn get_nodes_mut<'a>(&'a mut self) -> FormulaNodesGuard<'a, PId> {
        FormulaNodesGuard {
            formula: self,
        }
    }

    /// Can return subnormal values
    pub fn calculate<P: Parameters<PId>>(&self, parameters: &P) -> f32 {
        // calculate_array seems to be always faster
        self.calculate_array(parameters)
    }

    pub fn calculate_tree<P: Parameters<PId>>(&self, parameters: &P) -> f32 {
        self.calc_tree_inner(parameters, 0)
    }

    pub fn calculate_array<P: Parameters<PId>>(&self, parameters: &P) -> f32 {
        assert!(self.is_sorted);

        let mut results = vec![f32::NAN; self.nodes.len()];
        (0..self.nodes.len()).rev().for_each(|i| {
            results[i] = match &self.nodes[i] {
                FormulaNode::Value(v) => *v,
                FormulaNode::Parameter(id) => parameters.get_value(id),
                FormulaNode::Operation(op_node) => match op_node {
                    OpNode::Unary(unary_op, idx1) => unary_op.calc(results[*idx1]),
                    OpNode::Binary(binary_op, idx1, idx2) => {
                        binary_op.calc(results[*idx1], results[*idx2])
                    }
                },
            }
        });
        results[0]
    }

    // TODO: Optimize
    fn sort_nodes(&mut self) {
        self.compact();

        let mut true_idx = vec![0; self.nodes.len()];
        self.index_nodes(&mut true_idx, 0, 0);

        let mut new_nodes = vec![FormulaNode::Value(0.); self.nodes.len()];
        true_idx
            .iter()
            .enumerate()
            .for_each(|(old, new)| new_nodes[*new] = self.nodes[old].clone());
        self.nodes = new_nodes;

        self.nodes.iter_mut().for_each(|node| {
            if let FormulaNode::Operation(op_node) = node {
                match op_node {
                    OpNode::Unary(_, idx1) => {
                        *idx1 = true_idx[*idx1];
                    }
                    OpNode::Binary(_, idx1, idx2) => {
                        *idx1 = true_idx[*idx1];
                        *idx2 = true_idx[*idx2];
                    }
                }
            }
        });

        self.is_sorted = true;
    }

    fn index_nodes(
        &self,
        true_idx: &mut Vec<usize>,
        node_idx: usize,
        mut next_idx: usize,
    ) -> usize {
        true_idx[node_idx] = next_idx;
        next_idx += 1;
        if let FormulaNode::Operation(op_node) = &self.nodes[node_idx] {
            match op_node {
                OpNode::Unary(_, idx1) => {
                    next_idx = self.index_nodes(true_idx, *idx1, next_idx);
                }
                OpNode::Binary(_, idx1, idx2) => {
                    next_idx = self.index_nodes(true_idx, *idx1, next_idx);
                    next_idx = self.index_nodes(true_idx, *idx2, next_idx);
                }
            }
        }
        next_idx
    }

    fn calc_tree_inner<P: Parameters<PId>>(&self, parameters: &P, idx: usize) -> f32 {
        match &self.nodes[idx] {
            FormulaNode::Value(value) => *value,
            FormulaNode::Parameter(id) => parameters.get_value(id),
            FormulaNode::Operation(op_node) => match op_node {
                OpNode::Unary(unary_op, idx1) => {
                    unary_op.calc(self.calc_tree_inner(parameters, *idx1))
                }
                OpNode::Binary(binary_op, idx1, idx2) => binary_op.calc(
                    self.calc_tree_inner(parameters, *idx1),
                    self.calc_tree_inner(parameters, *idx2),
                ),
            },
        }
    }

    fn traverse_inner(&mut self, f: &mut Vec<bool>, idx: usize) {
        f[idx] = true;
        match self.nodes[idx] {
            FormulaNode::Value(_) => {}
            FormulaNode::Parameter(_) => {}
            FormulaNode::Operation(op_node) => match op_node {
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

    fn compact(&mut self) {
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
        self.nodes.iter_mut().for_each(|node| {
            if let FormulaNode::Operation(op_node) = node {
                match op_node {
                    OpNode::Unary(_, idx1) => {
                        *idx1 = new_idx[*idx1];
                    }
                    OpNode::Binary(_, idx1, idx2) => {
                        *idx1 = new_idx[*idx1];
                        *idx2 = new_idx[*idx2];
                    }
                }
            }
        });
    }

    fn get_subformula(&self, idx: usize) -> String {
        match &self.nodes[idx] {
            FormulaNode::Value(value) => format!("{:.2}", value),
            FormulaNode::Parameter(id) => id.get_name().clone(),
            FormulaNode::Operation(op_node) => match op_node {
                OpNode::Unary(unary_op, idx1) => match unary_op {
                    UnaryOp::Sqr => format!("{}^2", self.get_subformula(*idx1)),
                    UnaryOp::Sqrt => format!("sqrt({})", self.get_subformula(*idx1)),
                    UnaryOp::Pow(n) => format!("({})^{}", self.get_subformula(*idx1), n),
                    UnaryOp::Powi(n) => format!("({})^{}", self.get_subformula(*idx1), n),
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

pub mod rng {
    use super::*;
    use rand::{
        Rng, RngExt,
        distr::{Distribution, StandardUniform},
    };

    impl Distribution<UnaryOp> for StandardUniform {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> UnaryOp {
            match rng.random_range(0..5) {
                0 => UnaryOp::Sqr,
                1 => UnaryOp::Sqrt,
                2 => UnaryOp::Ln,
                3 => UnaryOp::Inv,
                4 => UnaryOp::Minus,
                _ => panic!(),
            }
        }
    }

    impl Distribution<BinaryOp> for StandardUniform {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BinaryOp {
            match rng.random_range(0..4) {
                0 => BinaryOp::Add,
                1 => BinaryOp::Sub,
                2 => BinaryOp::Div,
                3 => BinaryOp::Mul,
                _ => panic!(),
            }
        }
    }

    impl<PId: ParameterId> FormulaNode<PId>
    where
        StandardUniform: Distribution<PId>,
    {
        pub fn generate<R: Rng + ?Sized>(
            rng: &mut R,
            next_node_idx: usize,
            allow_op: bool,
        ) -> (Self, Vec<Self>) {
            match rng.random_range(if allow_op { 0..=2 } else { 0..=1 }) {
                0 => (Self::Value((rng.random::<f32>() - 0.5) * 2.), vec![]),
                1 => (Self::Parameter(rng.random::<PId>()), vec![]),
                2 => match rng.random_range(0..=1) {
                    0 => (
                        Self::Operation(OpNode::Unary(rng.random::<UnaryOp>(), next_node_idx)),
                        vec![Self::generate(rng, next_node_idx, false).0],
                    ),
                    1 => Self::generate_operation(rng, next_node_idx),
                    _ => {
                        panic!("TreeNode OpNode generate unexpected value");
                    }
                },
                _ => {
                    panic!("TreeNode generate unexpected value");
                }
            }
        }

        pub fn generate_operation<R: Rng + ?Sized>(
            rng: &mut R,
            next_node_idx: usize,
        ) -> (Self, Vec<Self>) {
            match rng.random_range(0..=1) {
                0 => (
                    Self::Operation(OpNode::Unary(rng.random::<UnaryOp>(), next_node_idx)),
                    vec![Self::generate(rng, next_node_idx, false).0],
                ),
                1 => (
                    Self::Operation(OpNode::Binary(
                        rng.random::<BinaryOp>(),
                        next_node_idx,
                        next_node_idx + 1,
                    )),
                    vec![
                        Self::generate(rng, next_node_idx, false).0,
                        Self::generate(rng, next_node_idx + 1, false).0,
                    ],
                ),
                _ => {
                    panic!("TreeNode OpNode generate unexpected value");
                }
            }
        }
    }
}
