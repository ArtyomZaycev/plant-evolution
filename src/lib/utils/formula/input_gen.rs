use std::{
    collections::HashMap,
    hash::Hash,
    ops::{BitAnd, BitOr, Deref, Neg, Range, Sub},
};

use crate::utils::formula::{BinaryOp, FormulaNode, OpNode, ParameterId, UnaryOp};

#[derive(Debug, Clone, Copy)]
struct InputGuard {
    possible: bool,
    // true - positive, false - negative
    sign: Option<bool>,
    // true - only 0, false - never 0
    is_zero: Option<bool>,
}

impl InputGuard {
    const FREE: Self = Self {
        possible: true,
        sign: None,
        is_zero: None,
    };

    const POSITIVE: Self = Self {
        sign: Some(true),
        ..Self::FREE
    };

    const NEGATIVE: Self = Self {
        sign: Some(false),
        ..Self::FREE
    };

    const ZERO: Self = Self {
        is_zero: Some(true),
        ..Self::FREE
    };

    const NON_ZERO: Self = Self {
        is_zero: Some(false),
        ..Self::FREE
    };
}

impl BitAnd for InputGuard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        let possible = self.possible && rhs.possible && {
            (self.sign.is_none() || rhs.sign.is_none() || self.sign == rhs.sign)
                && (self.is_zero.is_none() || rhs.is_zero.is_none() || self.is_zero == rhs.is_zero)
        };
        Self {
            possible,
            sign: self.sign.or(rhs.sign),
            is_zero: self.is_zero.or(rhs.is_zero),
        }
    }
}

impl Neg for InputGuard {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            sign: self.sign.map(|v| !v),
            ..self
        }
    }
}

#[derive(Debug, Clone)]
struct GuardRange {
    min_inclusive: bool,
    min: f32,
    max_inclusive: bool,
    max: f32,
}

impl Default for GuardRange {
    fn default() -> Self {
        Self {
            min_inclusive: false,
            min: f32::NEG_INFINITY,
            max_inclusive: false,
            max: f32::INFINITY,
        }
    }
}

impl GuardRange {
    const ZERO: Self = Self {
        min_inclusive: true,
        min: 0.,
        max_inclusive: true,
        max: 0.,
    };

    fn is_min_bound(&self) -> bool {
        self.min.is_finite()
    }

    fn is_max_bound(&self) -> bool {
        self.max.is_finite()
    }
}

#[derive(Debug, Clone)]
struct Guard2 {
    ranges: Vec<GuardRange>,
}

impl BitAnd<Guard2> for Guard2 {
    type Output = Guard2;

    fn bitand(self, rhs: Guard2) -> Self::Output {
        todo!()
    }
}

pub struct InputGuards<PId: ParameterId + Hash + Eq> {
    guards: HashMap<PId, InputGuard>,
}

impl<PId: ParameterId + Hash + Eq> InputGuards<PId> {
    pub fn new(nodes: Vec<FormulaNode<PId>>) -> Self {
        let mut guards = HashMap::new();
        Self::populate(&mut guards, InputGuard::FREE, &nodes, 0);
        Self { guards }
    }

    fn populate(
        guards: &mut HashMap<PId, InputGuard>,
        guard: InputGuard,
        nodes: &Vec<FormulaNode<PId>>,
        idx: usize,
    ) {
        match &nodes[idx] {
            FormulaNode::Value(_) => {}
            FormulaNode::Parameter(id) => {
                guards.insert(id.clone(), guard);
            }
            FormulaNode::Operation(op_node) => match op_node {
                OpNode::Unary(unary_op, idx1) => match unary_op {
                    UnaryOp::Sqr => Self::populate(guards, guard, nodes, *idx1),
                    UnaryOp::Sqrt => {
                        Self::populate(guards, guard & InputGuard::POSITIVE, nodes, *idx1)
                    }
                    UnaryOp::Ln => Self::populate(
                        guards,
                        guard & InputGuard::POSITIVE & InputGuard::NON_ZERO,
                        nodes,
                        *idx1,
                    ),
                    UnaryOp::Inv => {
                        Self::populate(guards, guard & InputGuard::NON_ZERO, nodes, *idx1)
                    }
                    UnaryOp::Minus => Self::populate(guards, -guard, nodes, *idx1),
                },
                OpNode::Binary(binary_op, idx1, idx2) => match binary_op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Pow
                    | BinaryOp::Powi => {
                        Self::populate(guards, guard, nodes, *idx1);
                        Self::populate(guards, guard, nodes, *idx2);
                    }
                    BinaryOp::Div => {
                        Self::populate(guards, guard, nodes, *idx1);
                        Self::populate(guards, guard & InputGuard::NON_ZERO, nodes, *idx2);
                    }
                },
            },
        }
    }
}
