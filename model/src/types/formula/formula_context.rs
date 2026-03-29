use crate::types::formula::{FormulaModel, FormulaModelContext, FormulaVariable};
use bevy_reflect::TypePath;

#[derive(TypePath)]
pub struct UnitFormulaContext;

pub type UnitFormulaModel = FormulaModel<UnitFormulaContext>;

impl FormulaModelContext for UnitFormulaContext {
    fn validate_variable(_var: &FormulaVariable) -> rootcause::Result<()> {
        // all variables are valid in this scope
        Ok(())
    }

    fn description() -> String {
        "fn() -> f64\nIds refer to unit variables by default".to_string()
    }

    fn default_namespace() -> Option<&'static str> {
        Some("unit")
    }
}
