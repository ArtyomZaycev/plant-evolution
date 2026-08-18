use serde::{Deserialize, Serialize};
use std::{any::type_name, fmt::Debug};

use super::parameters::*;

pub trait FormulaRuntime<PId: ParameterId>: Debug {
    fn calculate(&self, nodes: &Vec<FormulaNode<PId>>, parameters: &dyn Parameters<PId>) -> f32;
    fn update(&mut self, nodes: &mut Vec<FormulaNode<PId>>);

    fn get_name(&self) -> String;
}

pub trait DynamicFormulaRuntime<PId: ParameterId>: FormulaRuntime<PId> {
    fn swap<NR: FormulaRuntime<PId> + 'static>(&mut self, nr: NR);
}

pub trait BuildableRuntime<PId: ParameterId>: FormulaRuntime<PId> {
    fn new(nodes: &mut Vec<FormulaNode<PId>>) -> Self;
}

impl<PId: ParameterId, R: FormulaRuntime<PId> + Default> BuildableRuntime<PId> for R {
    fn new(_: &mut Vec<FormulaNode<PId>>) -> Self {
        Self::default()
    }
}

mod naive {
    use super::*;

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub struct NaiveRuntime {}

    impl NaiveRuntime {
        fn calculate_inner<PId: ParameterId>(
            &self,
            nodes: &Vec<FormulaNode<PId>>,
            parameters: &dyn Parameters<PId>,
            idx: usize,
        ) -> f32 {
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

    impl<PId: ParameterId> FormulaRuntime<PId> for NaiveRuntime {
        fn calculate(
            &self,
            nodes: &Vec<FormulaNode<PId>>,
            parameters: &dyn Parameters<PId>,
        ) -> f32 {
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
    pub struct ArrayRuntime {}

    impl ArrayRuntime {
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

    impl<PId: ParameterId> BuildableRuntime<PId> for ArrayRuntime {
        fn new(nodes: &mut Vec<FormulaNode<PId>>) -> Self {
            Self::sort_nodes(nodes);
            Self {}
        }
    }

    impl<PId: ParameterId> FormulaRuntime<PId> for ArrayRuntime {
        fn calculate(
            &self,
            nodes: &Vec<FormulaNode<PId>>,
            parameters: &dyn Parameters<PId>,
        ) -> f32 {
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

mod boxed {
    use super::*;

    #[derive(Debug)]
    pub struct BoxedRuntime<PId: ParameterId> {
        runtime: Box<dyn FormulaRuntime<PId>>,
    }

    impl<PId: ParameterId> BoxedRuntime<PId> {
        pub fn new<R: FormulaRuntime<PId> + 'static>(runtime: R) -> Self {
            Self {
                runtime: Box::new(runtime),
            }
        }
    }

    impl<PId: ParameterId> FormulaRuntime<PId> for BoxedRuntime<PId> {
        fn calculate(
            &self,
            nodes: &Vec<FormulaNode<PId>>,
            parameters: &dyn Parameters<PId>,
        ) -> f32 {
            self.runtime.calculate(nodes, parameters)
        }

        fn update(&mut self, nodes: &mut Vec<FormulaNode<PId>>) {
            self.runtime.update(nodes);
        }

        fn get_name(&self) -> String {
            self.runtime.get_name()
        }
    }

    impl<PId: ParameterId> DynamicFormulaRuntime<PId> for BoxedRuntime<PId> {
        fn swap<NR: FormulaRuntime<PId> + 'static>(&mut self, nr: NR) {
            self.runtime = Box::new(nr);
        }
    }
}

pub use array::*;
pub use boxed::*;
pub use naive::*;
