use std::fmt::Debug;

use crate::utils::formula::{Formula, ParameterId, Parameters, TreeArrayFormula, TreeFormula};

pub struct BoxedFormula<P> {
    formula: Box<dyn Formula<P>>,
}

impl<P> BoxedFormula<P> {
    pub fn new<F: Formula<P> + 'static>(formula: F) -> Self {
        Self {
            formula: Box::new(formula),
        }
    }
}

impl<P> Debug for BoxedFormula<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxedFormula")
            .field("formula", &self.formula)
            .finish()
    }
}

impl<P> ToString for BoxedFormula<P> {
    fn to_string(&self) -> String {
        format!("Boxed{}", self.formula.to_string())
    }
}

impl<P> Formula<P> for BoxedFormula<P> {
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
