use bevy::ecs::system::SystemParam;
use bevy::prelude::{Component, Reflect, Res};
use mod_loading::mods::ModData;
use model::registries::variable::UnitVariableModel;
use model::types::formula::formula_context::UnitFormulaContext;
use model::types::formula::formula_executor::FormulaExecutor;
use model::types::formula::{FormulaModelArgs, FormulaVariable};
use registry::registry::id::IdRef;
use registry::registry::reflect_registry::ReflectRegistry;
use utils::map::HashMap;

#[derive(Debug, Clone, Default, Reflect, Component)]
pub struct UnitVariables {
    vars: HashMap<IdRef<UnitVariableModel>, VarData>,
}

#[derive(Debug, Clone, Default, Reflect, Component)]
struct VarData {
    value: f64,
    readonly: bool,
}

impl UnitVariables {
    #[must_use]
    pub fn get(&self, reg: &ReflectRegistry, id: IdRef<UnitVariableModel>) -> f64 {
        if let Some(var) = self.vars.get(&id) {
            var.value
        } else {
            reg[id].default_value
        }
    }

    /// Returns a formula executor for this unit
    #[must_use]
    pub fn executor<'a, 'w>(
        &'a self,
        ctx: &'w CombatFormulaContext<'w>,
    ) -> UnitFormulaExecutor<'a, 'w> {
        UnitFormulaExecutor { vars: self, ctx }
    }
}

#[derive(SystemParam)]
pub struct CombatFormulaContext<'w> {
    mod_data: Res<'w, ModData>,
}

pub struct UnitFormulaExecutor<'a, 'w> {
    vars: &'a UnitVariables,
    ctx: &'w CombatFormulaContext<'w>,
}

impl<ARGS: FormulaModelArgs> FormulaExecutor<ARGS, UnitFormulaContext<ARGS>>
    for UnitFormulaExecutor<'_, '_>
{
    fn resolve_variable(&self, var: &FormulaVariable) -> rootcause::Result<f64> {
        match var {
            FormulaVariable::UnitVariable { id_ref } => {
                Ok(self.vars.get(&self.ctx.mod_data.registry, *id_ref))
            }
            FormulaVariable::Local(_) => {
                unreachable!()
            }
        }
    }
}
