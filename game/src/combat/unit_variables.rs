use crate::combat::CombatVariablesUpdate;
use bevy::app::{App, Plugin};
use bevy::ecs::system::SystemParam;
use bevy::log::error;
use bevy::prelude::{Component, Query, Reflect, Res};
use mod_loading::mods::ModData;
use model::registries::variable::UnitVariableModel;
use model::types::formula::formula_context::UnitFormulaContext;
use model::types::formula::formula_executor::FormulaExecutor;
use model::types::formula::{FormulaModelArgs, FormulaVariable};
use registry::registry::id::IdRef;
use registry::registry::reflect_registry::ReflectRegistry;
use utils::map::HashMap;

pub struct UnitVariablesPlugin;

impl Plugin for UnitVariablesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(CombatVariablesUpdate, sys_progress_variables);
    }
}

fn sys_progress_variables(
    mod_data: Res<ModData>,
    query: Query<(&mut UnitVariables, &mut UnitVariablesChanges)>,
) {
    for (mut vars, mut write) in query {
        for (id, add) in write.unit.drain() {
            vars.add(&mod_data.registry, id, add);
        }
    }
}
#[derive(Debug, Clone, Default, Reflect, Component)]
#[require(UnitVariablesChanges)]
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
    /// Creates a new [UnitVariables] with the given preset values
    pub fn new(reg: &ReflectRegistry, preset: &HashMap<IdRef<UnitVariableModel>, f64>) -> Self {
        let mut vars = HashMap::default();
        for (id, value) in preset {
            let var = &reg[*id];
            vars.insert(
                *id,
                VarData {
                    value: *value,
                    readonly: var.readonly,
                },
            );
        }
        Self { vars }
    }

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

    fn add(&mut self, reg: &ReflectRegistry, id: IdRef<UnitVariableModel>, value: f64) {
        let entry = self.vars.entry(id).or_insert_with(|| {
            let var = &reg[id];
            VarData {
                value: var.default_value,
                readonly: var.readonly,
            }
        });

        if entry.readonly {
            error!("Attempted to modify readonly variable {}. Ignoring.", id);
        } else {
            entry.value += value;
        }
    }
}

#[derive(Debug, Clone, Default, Reflect, Component)]
struct UnitVariablesChanges {
    unit: HashMap<IdRef<UnitVariableModel>, f64>,
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
