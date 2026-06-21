use crate::types::formula::{FormulaModel, FormulaModelArgs, FormulaModelContext, FormulaVariable};
use rootcause::prelude::ResultExt as _;
use rootcause::report;

pub trait FormulaExecutor<ARGS: FormulaModelArgs, CTX: FormulaModelContext<ARGS>> {
    /// Resolves the value of a variable in the context of the formula execution
    fn resolve_variable(&self, var: &FormulaVariable) -> rootcause::Result<f64>;

    fn execute_formula(
        &self,
        formula: &FormulaModel<ARGS, CTX>,
        args: ARGS::Input,
    ) -> rootcause::Result<f64>
    where
        Self: Sized,
    {
        default_execute_formula(self, formula, args)
    }
}

pub fn default_execute_formula<
    ARGS: FormulaModelArgs,
    CTX: FormulaModelContext<ARGS>,
    EXEC: FormulaExecutor<ARGS, CTX>,
>(
    executor: &EXEC,
    formula: &FormulaModel<ARGS, CTX>,
    args: ARGS::Input,
) -> rootcause::Result<f64> {
    match formula {
        FormulaModel::Const(value) => Ok(*value),
        FormulaModel::Expr(expr) => {
            let arg_indices = ARGS::arguments_indices();
            let arg_values = ARGS::from_input(args);

            let mut variables = vec![];

            for var in &expr.args {
                match var {
                    FormulaVariable::Local(name) => {
                        let idx = arg_indices
                            .get(name)
                            .ok_or_else(|| report!("Argument '{}' not found", name))?;
                        let value = arg_values
                            .get(*idx)
                            .ok_or_else(|| report!("Argument index '{}' out of bounds", idx))?;
                        variables.push(*value);
                    }
                    var => {
                        let value = executor.resolve_variable(var)?;
                        variables.push(value);
                    }
                }
            }

            Ok(expr.expr.eval_vec(variables).context("Evaluation failed")?)
        }
    }
}
