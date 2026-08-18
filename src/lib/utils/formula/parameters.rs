use std::fmt::Debug;

use serde::{Deserialize, Serialize};

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
    pub fn calc(&self, v1: f32) -> f32 {
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

impl ParameterId for usize {
    fn get_name(&self) -> String {
        self.to_string()
    }
}

impl Parameters<usize> for &[f32] {
    fn get_value(&self, id: &usize) -> f32 {
        self[*id]
    }
}