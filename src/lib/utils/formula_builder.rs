use std::cell::RefCell;

use crate::utils::formula::*;

pub enum FormulaBuilderError {
    Invalid,
}

pub struct FormulaBuilder<PId: ParameterId> {
    nodes: RefCell<Vec<FormulaNode<PId>>>,
}

#[must_use]
pub struct FormulaBuilderNode<'a, PId: ParameterId> {
    builder: &'a FormulaBuilder<PId>,
    idx: usize,
}

impl<'a, PId: ParameterId> FormulaBuilderNode<'a, PId> {
    pub fn build(self) -> Formula<PId> {
        self.builder.nodes.borrow_mut().swap(0, self.idx);
        Formula::new(self.builder.nodes.take())
    }
}

impl<PId: ParameterId> FormulaBuilder<PId> {
    pub fn new() -> Self {
        Self {
            nodes: RefCell::new(vec![]),
        }
    }

    pub fn node<'a>(&'a self, node: FormulaNode<PId>) -> FormulaBuilderNode<'a, PId> {
        let mut nodes = self.nodes.borrow_mut();
        let idx = nodes.len();
        nodes.push(node);
        FormulaBuilderNode {
            builder: &self,
            idx,
        }
    }

    pub fn value<'a>(&'a self, value: f32) -> FormulaBuilderNode<'a, PId> {
        self.node(FormulaNode::Value(value))
    }

    pub fn parameter<'a>(&'a self, id: PId) -> FormulaBuilderNode<'a, PId> {
        self.node(FormulaNode::Parameter(id))
    }

    pub fn unary_operation<'a>(
        &'a self,
        operation: UnaryOp,
        first: FormulaBuilderNode<'a, PId>,
    ) -> FormulaBuilderNode<'a, PId> {
        self.node(FormulaNode::Operation(OpNode::Unary(operation, first.idx)))
    }

    pub fn binary_operator<'a>(
        &'a self,
        operation: BinaryOp,
        first: FormulaBuilderNode<'a, PId>,
        second: FormulaBuilderNode<'a, PId>,
    ) -> FormulaBuilderNode<'a, PId> {
        self.node(FormulaNode::Operation(OpNode::Binary(
            operation, first.idx, second.idx,
        )))
    }
}

#[allow(unused_imports, dead_code)]
mod test {
    use crate::utils::{formula::*, formula_builder::FormulaBuilder};

    #[derive(Debug, Clone, Copy)]
    #[repr(usize)]
    enum SimplePId {
        A = 0,
        B = 1,
        C = 2,
        D = 3,
    }

    impl ParameterId for SimplePId {
        fn get_name(&self) -> String {
            String::default()
        }
    }

    impl Parameters<SimplePId> for [f32; 4] {
        fn get_value(&self, id: &SimplePId) -> f32 {
            self[*id as usize]
        }
    }

    /// (a + b) / c * d
    #[test]
    pub fn test_simple() {
        let formula1 = Formula::new(vec![
            FormulaNode::Operation(OpNode::Binary(BinaryOp::Mul, 1, 2)),
            FormulaNode::Operation(OpNode::Binary(BinaryOp::Div, 3, 4)),
            FormulaNode::Parameter(SimplePId::D),
            FormulaNode::Operation(OpNode::Binary(BinaryOp::Add, 5, 6)),
            FormulaNode::Parameter(SimplePId::C),
            FormulaNode::Parameter(SimplePId::A),
            FormulaNode::Parameter(SimplePId::B),
        ]);

        let b = FormulaBuilder::new();
        let formula2 = b.binary_operator(
            BinaryOp::Mul,
            b.binary_operator(
                BinaryOp::Div,
                b.binary_operator(
                    BinaryOp::Add,
                    b.parameter(SimplePId::A),
                    b.parameter(SimplePId::B),
                ),
                b.parameter(SimplePId::C),
            ),
            b.parameter(SimplePId::D),
        )
        .build();

        let input = [1., 2., 3., 4.];
        assert_eq!(formula1.calculate(&input), formula2.calculate(&input));
    }
}
