use std::cell::RefCell;

use crate::utils::formula::ParameterId;

use super::*;

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
    pub fn build(self) -> Vec<FormulaNode<PId>> {
        let mut nodes = self.builder.nodes.take();
        nodes.swap(0, self.idx);
        nodes.iter_mut().for_each(|node| {
            if let FormulaNode::Operation(op_node) = node {
                match op_node {
                    OpNode::Unary(_, idx1) => {
                        if *idx1 == 0 {
                            *idx1 = self.idx;
                        } else if *idx1 == self.idx {
                            *idx1 = 0;
                        }
                    }
                    OpNode::Binary(_, idx1, idx2) => {
                        if *idx1 == 0 {
                            *idx1 = self.idx;
                        } else if *idx1 == self.idx {
                            *idx1 = 0;
                        }
                        if *idx2 == 0 {
                            *idx2 = self.idx;
                        } else if *idx2 == self.idx {
                            *idx2 = 0;
                        }
                    }
                }
            }
        });
        nodes
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
    use crate::utils::formula::{Formula, Parameters};

    use super::*;

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
        let formula1: TreeFormula<SimplePId> = TreeFormula::new(vec![
            FormulaNode::Operation(OpNode::Binary(BinaryOp::Mul, 1, 2)),
            FormulaNode::Operation(OpNode::Binary(BinaryOp::Div, 3, 4)),
            FormulaNode::Parameter(SimplePId::D),
            FormulaNode::Operation(OpNode::Binary(BinaryOp::Add, 5, 6)),
            FormulaNode::Parameter(SimplePId::C),
            FormulaNode::Parameter(SimplePId::A),
            FormulaNode::Parameter(SimplePId::B),
        ]);

        let b = FormulaBuilder::new();
        let formula2: TreeFormula<SimplePId> = TreeFormula::new(
            b.binary_operator(
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
            .build(),
        );

        let input = [1., 2., 3., 4.];
        assert_eq!(formula1.calculate(&input), formula2.calculate(&input));
    }

    // (a.powi(4) / b.sqrt() + (c^-1).ln() - d.powi(2)).sqrt()
    #[test]
    pub fn test_complex() {
        let formula1: TreeFormula<SimplePId> = TreeFormula::new(vec![
            /*0*/
            FormulaNode::Operation(OpNode::Unary(UnaryOp::Sqrt, 1)), // (a.powi(4) / b.sqrt() + (c^-1).ln() - d.powi(2)).SQRT()
            /*1*/
            FormulaNode::Operation(OpNode::Binary(BinaryOp::Sub, 2, 3)), // a.powi(4) / b.sqrt() + (c^-1).ln() SUB d.powi(2)
            /*2*/
            FormulaNode::Operation(OpNode::Binary(BinaryOp::Add, 4, 5)), // a.powi(4) / b.sqrt() ADD (c^-1).ln()
            /*3*/ FormulaNode::Operation(OpNode::Unary(UnaryOp::Sqr, 6)), // d.POWI(2)
            /*4*/
            FormulaNode::Operation(OpNode::Binary(BinaryOp::Div, 7, 8)), // a.powi(4) DIV b.sqrt()
            /*5*/ FormulaNode::Operation(OpNode::Unary(UnaryOp::Ln, 9)), // (c^-1).LN()
            /*6*/ FormulaNode::Parameter(SimplePId::D), // D
            /*7*/
            FormulaNode::Operation(OpNode::Unary(UnaryOp::Powi(4), 10)), // a.POWI(4)
            /*8*/ FormulaNode::Operation(OpNode::Unary(UnaryOp::Sqrt, 11)), // b.SQRT()
            /*9*/ FormulaNode::Operation(OpNode::Unary(UnaryOp::Inv, 12)), // c POW -1
            /*10*/ FormulaNode::Parameter(SimplePId::A), // A
            /*11*/ FormulaNode::Parameter(SimplePId::B), // B
            /*12*/ FormulaNode::Parameter(SimplePId::C), // C
        ]);

        let b = FormulaBuilder::new();
        let formula2: TreeFormula<SimplePId> = TreeFormula::new(
            b.unary_operation(
                UnaryOp::Sqrt,
                b.binary_operator(
                    BinaryOp::Sub,
                    b.binary_operator(
                        BinaryOp::Add,
                        b.binary_operator(
                            BinaryOp::Div,
                            b.unary_operation(UnaryOp::Powi(4), b.parameter(SimplePId::A)),
                            b.unary_operation(UnaryOp::Sqrt, b.parameter(SimplePId::B)),
                        ),
                        b.unary_operation(
                            UnaryOp::Ln,
                            b.unary_operation(UnaryOp::Inv, b.parameter(SimplePId::C)),
                        ),
                    ),
                    b.unary_operation(UnaryOp::Sqr, b.parameter(SimplePId::D)),
                ),
            )
            .build(),
        );

        let input = [111., 24., 3., 4.];
        assert_eq!(formula1.calculate(&input), formula2.calculate(&input));
    }
}
