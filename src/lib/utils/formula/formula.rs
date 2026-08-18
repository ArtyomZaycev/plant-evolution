use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use crate::utils::formula::*;

use super::parameters::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formula<PId: ParameterId, R: FormulaRuntime<PId> = NaiveRuntime> {
    nodes: Vec<FormulaNode<PId>>,
    runtime: R,
}

pub struct FormulaNodesGuard<'a, PId: ParameterId, R: FormulaRuntime<PId>> {
    formula: &'a mut Formula<PId, R>,
}

impl<'a, PId: ParameterId, R: FormulaRuntime<PId>> Deref for FormulaNodesGuard<'a, PId, R> {
    type Target = Vec<FormulaNode<PId>>;

    fn deref(&self) -> &Self::Target {
        &self.formula.nodes
    }
}

impl<'a, PId: ParameterId, R: FormulaRuntime<PId>> DerefMut for FormulaNodesGuard<'a, PId, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.formula.nodes
    }
}

impl<'a, PId: ParameterId, R: FormulaRuntime<PId>> Drop for FormulaNodesGuard<'a, PId, R> {
    fn drop(&mut self) {
        self.formula.runtime.update(&mut self.formula.nodes);
    }
}

impl<PId: ParameterId, R: FormulaRuntime<PId>> Formula<PId, R> {
    pub fn new(mut nodes: Vec<FormulaNode<PId>>) -> Self
    where
        R: BuildableRuntime<PId>,
    {
        assert!(!nodes.is_empty());
        Self::compact(&mut nodes);
        let runtime = R::new(&mut nodes);
        Self { nodes, runtime }
    }

    pub fn new_wr(mut nodes: Vec<FormulaNode<PId>>, runtime: R) -> Self {
        assert!(!nodes.is_empty());
        Self::compact(&mut nodes);
        Self { nodes, runtime }
    }

    pub fn with_runtime<NR: FormulaRuntime<PId> + BuildableRuntime<PId>>(self) -> Formula<PId, NR> {
        Formula::<PId, NR>::new(self.nodes)
    }

    pub fn with_custom_runtime<NR: FormulaRuntime<PId>>(self, new_runtime: NR) -> Formula<PId, NR> {
        Formula::<PId, NR>::new_wr(self.nodes, new_runtime)
    }

    pub fn update_runtime<F: FnOnce(&mut R)>(&mut self, f: F) {
        f(&mut self.runtime)
    }

    pub fn get_nodes(&self) -> &Vec<FormulaNode<PId>> {
        &self.nodes
    }

    pub fn get_nodes_mut<'a>(&'a mut self) -> FormulaNodesGuard<'a, PId, R> {
        FormulaNodesGuard { formula: self }
    }

    /// Can return subnormal values
    pub fn calculate<P: Parameters<PId>>(&self, parameters: &P) -> f32 {
        self.runtime.calculate(&self.nodes, parameters)
    }

    fn traverse_inner(nodes: &mut Vec<FormulaNode<PId>>, f: &mut Vec<bool>, idx: usize) {
        f[idx] = true;
        match nodes[idx] {
            FormulaNode::Value(_) => {}
            FormulaNode::Parameter(_) => {}
            FormulaNode::Operation(op_node) => match op_node {
                OpNode::Unary(_, idx1) => {
                    Self::traverse_inner(nodes, f, idx1);
                }
                OpNode::Binary(_, idx1, idx2) => {
                    Self::traverse_inner(nodes, f, idx1);
                    Self::traverse_inner(nodes, f, idx2);
                }
            },
        }
    }

    fn compact(nodes: &mut Vec<FormulaNode<PId>>) {
        let mut f = vec![false; nodes.len()];
        Self::traverse_inner(nodes, &mut f, 0);
        let mut new_idx = vec![0; nodes.len()];
        f.iter().enumerate().fold(0, |cnt, (i, v)| {
            new_idx[i] = i - cnt;
            if *v {
                cnt
            } else {
                nodes.remove(i - cnt);
                cnt + 1
            }
        });
        nodes.iter_mut().for_each(|node| {
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
