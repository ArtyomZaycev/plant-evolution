use std::{collections::HashMap, fmt::Debug};

use self_cell::self_cell;
use xprs::Xprs;

use crate::utils::formula::{Formula, ParameterIdAll, Parameters};

self_cell!(
    struct XprsOwned {
        owner: String,
        #[covariant]
        dependent: Xprs,
    }
);

pub struct XprsFormula<PId: ParameterIdAll> {
    inner: XprsOwned,
    ids: HashMap<String, PId>,
}

impl<PId: ParameterIdAll> Debug for XprsFormula<PId> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XprsFormula")
            .field("raw", self.inner.borrow_owner())
            .finish()
    }
}

impl<PId: ParameterIdAll> XprsFormula<PId> {
    pub fn new(formula: String) -> Result<Self, xprs::ParseError> {
        let inner = XprsOwned::try_new(formula, |formula| Xprs::try_from(formula.as_str()))?;
        let ids = {
            let xprs = inner.borrow_dependent();
            PId::get_all()
                .filter(|(name, _)| xprs.vars.contains(name.as_str()))
                .collect()
        };
        Ok(Self { inner, ids })
    }
}

impl<PId: ParameterIdAll> ToString for XprsFormula<PId> {
    fn to_string(&self) -> String {
        self.inner.borrow_owner().clone()
    }
}

impl<PId: ParameterIdAll, P: Parameters<PId>> Formula<P> for XprsFormula<PId> {
    fn calculate(&self, parameters: &P) -> f32 {
        let xprs = self.inner.borrow_dependent();

        let vars = xprs
            .vars
            .iter()
            .map(|&name| {
                (
                    name,
                    parameters.get_value(self.ids.get(name).unwrap()) as f64,
                )
            })
            .collect::<HashMap<_, _>>();

        match xprs.eval(&vars) {
            Ok(v) => v as f32,
            Err(_) => f32::NAN,
        }
    }
}

#[cfg(test)]
mod test {
    use approx::assert_relative_eq;

    use crate::utils::formula::ArrayIdx;

    use super::*;

    #[test]
    pub fn test() {
        let str_formula = "a + b + c * sqrt(a)";
        let formula: XprsFormula<ArrayIdx<3>> = XprsFormula::new(str_formula.to_string()).unwrap();

        let value = formula.calculate(&[6., 2., 3.]);

        assert_relative_eq!(6. + 2. + 3. * 6_f32.sqrt(), value, epsilon = 1e-6);
    }
}
