use mathexpr::{CompileError, Executable, Expr, Expression, ParseError};

use crate::utils::formula::{Formula, ParameterIdAll, Parameters};

// TODO: Remove ParameterIdAll
#[derive(Debug)]
pub struct MathexprFormula<PId: ParameterIdAll> {
    raw: String,
    order: Vec<PId>,
    executable: Executable,
}

#[derive(Debug, Clone)]
pub enum MathexprFormulaBuildError {
    ParseError(ParseError),
    CompileError(CompileError),
}

impl<PId: ParameterIdAll> MathexprFormula<PId> {
    pub fn new(formula: String) -> Result<Self, MathexprFormulaBuildError> {
        let expr = Expression::parse(&formula)
            .map_err(|err| MathexprFormulaBuildError::ParseError(err))?;

        let mut used_vars = Vec::new();
        Self::collect_ast_vars(expr.ast(), &mut used_vars);

        let used_vars = PId::get_all()
            .filter(|(name, _)| used_vars.contains(name))
            .collect::<Vec<_>>();

        let executable = expr
            .compile(
                &used_vars
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<&str>>(),
            )
            .map_err(|err| MathexprFormulaBuildError::CompileError(err))?;

        Ok(Self {
            raw: formula,
            order: used_vars.into_iter().map(|(_, id)| id).collect(),
            executable,
        })
    }

    fn collect_ast_vars(ast: &Expr, vars: &mut Vec<String>) {
        match ast {
            Expr::Variable(name) => vars.push(name.clone()),
            Expr::BinaryOp { op: _, left, right } => {
                Self::collect_ast_vars(&left, vars);
                Self::collect_ast_vars(&right, vars);
            }
            Expr::UnaryMinus(expr) => {
                Self::collect_ast_vars(&expr, vars);
            }
            Expr::FunctionCall { name: _, args } => {
                args.iter().for_each(|arg| {
                    Self::collect_ast_vars(arg, vars);
                });
            }
            _ => {}
        }
    }
}

impl<PId: ParameterIdAll> ToString for MathexprFormula<PId> {
    fn to_string(&self) -> String {
        self.raw.clone()
    }
}

impl<PId: ParameterIdAll, P: Parameters<PId>> Formula<P> for MathexprFormula<PId> {
    fn calculate(&self, parameters: &P) -> f32 {
        match self.executable.eval(
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
    use approx::assert_relative_eq;

    use crate::utils::formula::ArrayIdx;

    use super::*;

    #[test]
    pub fn test() {
        let str_formula = "a + b + c * -sqrt(a)";
        let formula: MathexprFormula<ArrayIdx<3>> =
            MathexprFormula::new(str_formula.to_string()).unwrap();

        let value = formula.calculate(&[6., 2., 3.]);

        assert_relative_eq!(6. + 2. + 3. * -6_f32.sqrt(), value, epsilon = 1e-6);
    }
}
