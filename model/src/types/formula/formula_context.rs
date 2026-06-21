use crate::types::formula::formula_args::EmptyArgs;
use crate::types::formula::{FormulaModel, FormulaModelArgs, FormulaModelContext, FormulaVariable};
use bevy_reflect::TypePath;
use std::marker::PhantomData;

#[derive(TypePath)]
pub struct UnitFormulaContext<ARGS: FormulaModelArgs>(PhantomData<fn() -> ARGS>);

pub type UnitFormulaModel<ARGS = EmptyArgs> = FormulaModel<ARGS, UnitFormulaContext<ARGS>>;

impl<ARGS: FormulaModelArgs> FormulaModelContext<ARGS> for UnitFormulaContext<ARGS> {
    fn validate_variable(_var: &FormulaVariable) -> rootcause::Result<()> {
        // all variables are valid in this scope
        Ok(())
    }

    fn description() -> String {
        format!(
            "fn({}) -> f64\nIds refer to unit variables by default",
            ARGS::argument_names().join(", ")
        )
    }

    fn default_namespace() -> Option<&'static str> {
        Some("unit")
    }
}
