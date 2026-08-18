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
        utils::compact(&mut nodes);
        let runtime = R::new(&mut nodes);
        Self { nodes, runtime }
    }

    pub fn new_wr(mut nodes: Vec<FormulaNode<PId>>, runtime: R) -> Self {
        assert!(!nodes.is_empty());
        utils::compact(&mut nodes);
        Self { nodes, runtime }
    }

    pub fn boxed(self) -> Formula<PId, BoxedRuntime<PId>>
    where
        R: 'static,
    {
        Formula::new_wr(self.nodes, BoxedRuntime::new(self.runtime))
    }

    pub fn with_runtime<NR: FormulaRuntime<PId> + BuildableRuntime<PId>>(self) -> Formula<PId, NR> {
        Formula::<PId, NR>::new(self.nodes)
    }

    pub fn with_custom_runtime<NR: FormulaRuntime<PId>>(self, new_runtime: NR) -> Formula<PId, NR> {
        Formula::<PId, NR>::new_wr(self.nodes, new_runtime)
    }

    pub fn swap_runtime<NR: FormulaRuntime<PId> + BuildableRuntime<PId> + 'static>(&mut self)
    where
        R: DynamicFormulaRuntime<PId> + 'static,
    {
        self.runtime.swap(NR::new(&mut self.nodes));
    }

    pub fn swap_custom_runtime<NR: FormulaRuntime<PId> + BuildableRuntime<PId> + 'static>(
        &mut self,
        new_runtime: NR,
    ) where
        R: DynamicFormulaRuntime<PId> + 'static,
    {
        self.runtime.swap(new_runtime);
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

    pub fn get_formula(&self) -> String {
        utils::get_formula(&self.nodes)
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
