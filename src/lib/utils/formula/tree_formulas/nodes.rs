use serde::{Deserialize, Serialize};

use crate::utils::formula::ParameterId;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UnaryOp {
    Sqr,
    Sqrt,
    Ln,
    Inv,
    Minus,
}

impl UnaryOp {
    pub fn calc(&self, v1: f32) -> f32 {
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
    Pow,
    Powi,
}

impl BinaryOp {
    pub fn calc(&self, v1: f32, v2: f32) -> f32 {
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
            BinaryOp::Pow => v1.powf(v2),
            BinaryOp::Powi => v1.powi(v2 as i32),
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
