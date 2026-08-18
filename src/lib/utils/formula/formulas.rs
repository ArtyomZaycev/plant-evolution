use std::fmt::Debug;

use crate::utils::formula::{Formula, ParameterId, Parameters, TreeArrayFormula, TreeFormula};

#[derive(Debug)]
pub struct BoxedFormula<P: Debug> {
    formula: Box<dyn Formula<P>>,
}

impl<P: Debug> ToString for BoxedFormula<P> {
    fn to_string(&self) -> String {
        format!("Boxed{}", self.formula.to_string())
    }
}

impl<P: Debug> Formula<P> for BoxedFormula<P> {
    fn calculate(&self, parameters: &P) -> f32 {
        self.formula.calculate(parameters)
    }
}

#[derive(Debug)]
pub enum EnumFormula<PId: ParameterId + Debug> {
    Tree(TreeFormula<PId>),
    TreeArray(TreeArrayFormula<PId>),
}

impl<PId: ParameterId + Debug> ToString for EnumFormula<PId> {
    fn to_string(&self) -> String {
        format!(
            "Enum{}",
            match self {
                EnumFormula::Tree(f) => f.to_string(),
                EnumFormula::TreeArray(f) => f.to_string(),
            }
        )
    }
}

impl<PId: ParameterId + Debug, P: Parameters<PId>> Formula<P> for EnumFormula<PId> {
    fn calculate(&self, parameters: &P) -> f32 {
        match self {
            EnumFormula::Tree(f) => f.calculate(parameters),
            EnumFormula::TreeArray(f) => f.calculate(parameters),
        }
    }
}
