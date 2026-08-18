use std::{collections::HashMap, fmt::Debug};

use crate::utils::formula::{Formula, ParameterIdAll, Parameters};

struct BaseContext<'a, PId: ParameterIdAll> {
    ids_map: HashMap<String, PId>,
    functions: HashMap<String, Box<dyn Fn(&[f64]) -> f64 + 'a>>,
}

impl<PId: ParameterIdAll> Debug for BaseContext<'_, PId> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BaseContext")
            .field("ids_map", &self.ids_map)
            .field(
                "functions",
                &self
                    .functions
                    .iter()
                    .map(|(name, _)| name)
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl<'a, PId: ParameterIdAll> BaseContext<'a, PId> {
    fn new() -> Self {
        Self {
            ids_map: PId::get_all().collect(),
            functions: Self::create_functions(),
        }
    }

    fn create_functions() -> HashMap<String, Box<dyn Fn(&[f64]) -> f64 + 'a>> {
        let mut functions = HashMap::new();
        let f: Box<dyn Fn(&[f64]) -> f64 + 'a> = Box::new(|v: &[f64]| v[0].sqrt());
        functions.insert("sqrt".to_owned(), f);

        functions
    }

    fn fill<'b, P: Parameters<PId>>(&'b self, parameters: &'b P) -> Context<'b, PId, P> {
        Context {
            base: &self,
            parameters: parameters,
        }
    }
}
struct Context<'a, PId: ParameterIdAll, P: Parameters<PId>> {
    base: &'a BaseContext<'a, PId>,
    parameters: &'a P,
}

impl<'a, PId: ParameterIdAll, P: Parameters<PId>> meval::ContextProvider for Context<'a, PId, P> {
    fn get_var(&self, name: &str) -> Option<f64> {
        Some(self.parameters.get_value(self.base.ids_map.get(name)?) as f64)
    }
    fn eval_func(&self, name: &str, args: &[f64]) -> Result<f64, meval::FuncEvalError> {
        self.base
            .functions
            .get(name)
            .map_or(Err(meval::FuncEvalError::UnknownFunction), |f| Ok(f(args)))
    }
}

#[derive(Debug)]
pub struct MEvalFormula<'a, PId: ParameterIdAll> {
    raw: String,
    base_context: BaseContext<'a, PId>,
    expr: meval::Expr,
}

impl<PId: ParameterIdAll> MEvalFormula<'_, PId> {
    pub fn new(formula: String) -> Result<Self, meval::Error> {
        let expr = formula.parse()?;

        Ok(Self {
            raw: formula,
            base_context: BaseContext::new(),
            expr,
        })
    }
}

impl<PId: ParameterIdAll> ToString for MEvalFormula<'_, PId> {
    fn to_string(&self) -> String {
        self.raw.clone()
    }
}

impl<PId: ParameterIdAll, P: Parameters<PId>> Formula<P> for MEvalFormula<'_, PId> {
    fn calculate(&self, parameters: &P) -> f32 {
        let context = self.base_context.fill(parameters);
        match self.expr.eval_with_context(context) {
            Ok(v) => v as f32,
            Err(_) => f32::NAN,
        }
    }
}

#[allow(dead_code, unused_imports)]
mod test {
    use crate::utils::formula::ArrayIdx;

    use super::*;

    #[test]
    pub fn test() {
        let str_formula = "a + b + c * sqrt(a)";
        let formula: MEvalFormula<'_, ArrayIdx<3>> =
            MEvalFormula::new(str_formula.to_string()).unwrap();

        let parameters: &[f32; 3] = &[1., 2., 3.];
        let value = formula.calculate(parameters);

        assert_eq!(1. + 2. + 3. * 1_f32.sqrt(), value)
    }
}
