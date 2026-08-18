use std::{collections::HashMap, hash::Hash};

use tabulon::{CompiledExpr, Parser, Tabula, VarResolver};

use crate::utils::formula::{Formula, ParameterIdAll, Parameters};

struct ParameterResolver<PId: ParameterIdAll> {
    ids_map: HashMap<String, PId>,
}

impl<PId: ParameterIdAll> ParameterResolver<PId> {
    fn new() -> Self {
        Self {
            ids_map: PId::get_all().collect(),
        }
    }
}

impl<PId: ParameterIdAll> VarResolver<PId> for ParameterResolver<PId> {
    fn resolve(&self, ident: &str) -> Result<PId, tabulon::VarResolveError> {
        self.ids_map
            .get(ident)
            .cloned()
            .ok_or(tabulon::VarResolveError::Unknown(ident.to_owned()))
    }
}

#[derive(Debug)]
pub struct TabulonFormula<PId: ParameterIdAll> {
    raw: String,
    expression: CompiledExpr<PId>,
    order: Vec<PId>,
}

impl<PId: ParameterIdAll> ToString for TabulonFormula<PId> {
    fn to_string(&self) -> String {
        self.raw.clone()
    }
}

impl<PId: ParameterIdAll + Eq + Hash> TabulonFormula<PId> {
    pub fn new(formula: String) -> Result<Self, tabulon::JitError> {
        let mut engine = Tabula::new_ctx();
        let resolver = ParameterResolver::<PId>::new();

        let prepared = Parser::new(&formula)?.parse_with_var_resolver(&resolver)?;
        let expression = engine.compile_prepared(&prepared)?;

        Ok(Self {
            raw: formula,
            order: expression.vars().to_vec(),
            expression,
        })
    }
}

impl<PId: ParameterIdAll + Eq + Hash, P: Parameters<PId>> Formula<P> for TabulonFormula<PId> {
    fn calculate(&self, parameters: &P) -> f32 {
        match self.expression.eval(
            &self
                .order
                .iter()
                .map(|id| parameters.get_value(id) as f64)
                .collect::<Vec<_>>(),
        ) {
            Ok(v) => v as f32,
            Err(_) => f32::NAN,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::utils::formula::ArrayIdx;

    use super::*;

    #[test]
    pub fn test() {
        let str_formula = "a + b + c * -a";
        let formula: TabulonFormula<ArrayIdx<3>> =
            TabulonFormula::new(str_formula.to_string()).unwrap();

        let value = formula.calculate(&[1., 2., 3.]);

        assert_eq!(1. + 2. + 3. * -1., value);
    }
}
