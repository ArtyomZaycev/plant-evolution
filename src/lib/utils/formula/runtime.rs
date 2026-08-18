use std::{any::type_name, fmt::Debug};
use serde::{Deserialize, Serialize};

use super::parameters::*;

pub trait FormulaRuntime<PId: ParameterId>: Debug + Clone {
    fn new(nodes: &mut Vec<FormulaNode<PId>>) -> Self;
    fn calculate<P: Parameters<PId>>(&self, nodes: &Vec<FormulaNode<PId>>, parameters: &P) -> f32;
    fn update(&mut self, nodes: &mut Vec<FormulaNode<PId>>);

    fn get_name(&self) -> String;
}

mod naive {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NaiveFormulaRuntime {}

    impl NaiveFormulaRuntime {
        fn calculate_inner<PId: ParameterId, P: Parameters<PId>>(&self, nodes: &Vec<FormulaNode<PId>>, parameters: &P, idx: usize) -> f32 {
            match &nodes[idx] {
                FormulaNode::Value(value) => *value,
                FormulaNode::Parameter(id) => parameters.get_value(id),
                FormulaNode::Operation(op_node) => match op_node {
                    OpNode::Unary(unary_op, idx1) => {
                        unary_op.calc(self.calculate_inner(nodes, parameters, *idx1))
                    }
                    OpNode::Binary(binary_op, idx1, idx2) => binary_op.calc(
                        self.calculate_inner(nodes, parameters, *idx1),
                        self.calculate_inner(nodes, parameters, *idx2),
                    ),
                },
            }
        }
    }

    impl<PId: ParameterId> FormulaRuntime<PId> for NaiveFormulaRuntime {
        fn new(_: &mut Vec<FormulaNode<PId>>) -> Self {
            Self {}
        }

        fn calculate<P: Parameters<PId>>(&self, nodes: &Vec<FormulaNode<PId>>, parameters: &P) -> f32 {
            self.calculate_inner(nodes, parameters, 0)
        }

        fn update(&mut self, _: &mut Vec<FormulaNode<PId>>) {}

        fn get_name(&self) -> String {
            type_name::<Self>().to_owned()
        }
    }
}

mod array {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ArrayFormulaRuntime {}

    impl ArrayFormulaRuntime {
        fn sort_nodes<PId: ParameterId>(nodes: &mut Vec<FormulaNode<PId>>) {
            let mut true_idx = vec![0; nodes.len()];
            Self::index_nodes(nodes, &mut true_idx, 0, 0);

            let mut new_nodes = vec![FormulaNode::Value(0.); nodes.len()];
            true_idx
                .iter()
                .enumerate()
                .for_each(|(old, new)| new_nodes[*new] = nodes[old].clone());
            *nodes = new_nodes;

            nodes.iter_mut().for_each(|node| {
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
        }
        
        fn index_nodes<PId: ParameterId>(
            nodes: &Vec<FormulaNode<PId>>,
            true_idx: &mut Vec<usize>,
            node_idx: usize,
            mut next_idx: usize,
        ) -> usize {
            true_idx[node_idx] = next_idx;
            next_idx += 1;
            if let FormulaNode::Operation(op_node) = &nodes[node_idx] {
                match op_node {
                    OpNode::Unary(_, idx1) => {
                        next_idx = Self::index_nodes(nodes, true_idx, *idx1, next_idx);
                    }
                    OpNode::Binary(_, idx1, idx2) => {
                        next_idx = Self::index_nodes(nodes, true_idx, *idx1, next_idx);
                        next_idx = Self::index_nodes(nodes, true_idx, *idx2, next_idx);
                    }
                }
            }
            next_idx
        }
    }

    impl<PId: ParameterId> FormulaRuntime<PId> for ArrayFormulaRuntime {
        fn new(nodes: &mut Vec<FormulaNode<PId>>) -> Self {
            Self::sort_nodes(nodes);
            Self { }
        }

        fn calculate<P: Parameters<PId>>(&self, nodes: &Vec<FormulaNode<PId>>, parameters: &P) -> f32 {
            let mut results = vec![f32::NAN; nodes.len()];
            (0..nodes.len()).rev().for_each(|i| {
                results[i] = match &nodes[i] {
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

        fn update(&mut self, nodes: &mut Vec<FormulaNode<PId>>) {
            Self::sort_nodes(nodes);
        }

        fn get_name(&self) -> String {
            type_name::<Self>().to_owned()
        }
    }
}

pub use naive::*;
pub use array::*;