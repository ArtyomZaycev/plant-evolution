use serde::{Deserialize, Serialize};

use crate::utils::formula::{tree_formula::utils, *};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeFormula<PId: ParameterId> {
    pub nodes: Vec<FormulaNode<PId>>,
}

impl<PId: ParameterId> TreeFormula<PId> {
    pub fn new(mut nodes: Vec<FormulaNode<PId>>) -> Self {
        assert!(!nodes.is_empty());
        utils::compact_nodes(&mut nodes);
        Self { nodes }
    }

    fn calculate_inner<P: Parameters<PId>>(
        &self,
        nodes: &Vec<FormulaNode<PId>>,
        parameters: &P,
        idx: usize,
    ) -> f32 {
        match &nodes[idx] {
            FormulaNode::Value(value) => *value,
            FormulaNode::Parameter(id) => parameters.get_value(id),
            FormulaNode::Operation(op_node) => match op_node {
                OpNode::Unary(unary_op, idx1) => {
                    unary_op.calc(self.calculate_inner::<P>(nodes, parameters, *idx1))
                }
                OpNode::Binary(binary_op, idx1, idx2) => binary_op.calc(
                    self.calculate_inner::<P>(nodes, parameters, *idx1),
                    self.calculate_inner::<P>(nodes, parameters, *idx2),
                ),
            },
        }
    }
}

impl<PId: ParameterId> ToString for TreeFormula<PId> {
    fn to_string(&self) -> String {
        utils::nodes_to_string(&self.nodes)
    }
}

impl<PId: ParameterId, P: Parameters<PId>> Formula<P> for TreeFormula<PId> {
    fn calculate(&self, parameters: &P) -> f32 {
        self.calculate_inner::<P>(&self.nodes, parameters, 0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeArrayFormula<PId: ParameterId>(TreeFormula<PId>);

impl<PId: ParameterId> TreeArrayFormula<PId> {
    pub fn new(mut nodes: Vec<FormulaNode<PId>>) -> Self {
        assert!(!nodes.is_empty());
        utils::compact_nodes(&mut nodes);
        utils::sort_nodes(&mut nodes);
        Self(TreeFormula::new(nodes))
    }
}

impl<PId: ParameterId> ToString for TreeArrayFormula<PId> {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl<PId: ParameterId, P: Parameters<PId>> Formula<P> for TreeArrayFormula<PId> {
    fn calculate(&self, parameters: &P) -> f32 {
        let mut results = vec![f32::NAN; self.0.nodes.len()];
        (0..self.0.nodes.len()).rev().for_each(|i| {
            results[i] = match &self.0.nodes[i] {
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
