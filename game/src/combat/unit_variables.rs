use crate::combat::CombatVariablesUpdate;
use bevy::app::{App, Plugin};
use bevy::ecs::system::SystemParam;
use bevy::log::error;
use bevy::prelude::{Component, Query, Reflect, Res};
use mod_loading::mods::ModData;
use model::registries::variable::{
    UnitVariableMap, UnitVariableModel, VariableValue, VariableValueDamage,
};
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
        for (id, change) in write.unit.drain() {
            vars.apply(&mod_data.registry, id, &change);
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
    value: VariableValue,
    damage: VariableValueDamage,
    computed: f64,
    readonly: bool,
}

impl UnitVariables {
    #[must_use]
    /// Creates a new [UnitVariables] with the given preset values
    pub fn new(reg: &ReflectRegistry, preset: &UnitVariableMap) -> Self {
        let mut vars = HashMap::default();
        for (id, value) in preset {
            let var = &reg[*id];
            vars.insert(
                *id,
                VarData::new(*value, VariableValueDamage::ZERO, var.readonly),
            );
        }
        Self { vars }
    }

    #[must_use]
    pub fn get(&self, reg: &ReflectRegistry, id: IdRef<UnitVariableModel>) -> f64 {
        if let Some(var) = self.vars.get(&id) {
            var.computed
        } else {
            reg[id].default_value.compute()
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

    fn apply(
        &mut self,
        reg: &ReflectRegistry,
        id: IdRef<UnitVariableModel>,
        value: &VariableValueChange,
    ) {
        let entry = self.vars.entry(id).or_insert_with(|| {
            let var = &reg[id];
            VarData::new(
                var.default_value,
                VariableValueDamage::ZERO,
                var.readonly,
            )
        });

        if entry.readonly {
            error!("Attempted to modify readonly variable {}. Ignoring.", id);
        } else {
            entry.change(value);
        }
    }
}

impl VarData {
    fn new(value: VariableValue, damage: VariableValueDamage, readonly: bool) -> Self {
        let mut val = Self {
            value,
            damage,
            computed: 0.0,
            readonly,
        };
        val.compute();
        val
    }
    fn compute(&mut self) {
        self.computed = self.value.compute() * self.damage.multiplier();
    }

    fn change(&mut self, change: &VariableValueChange) {
        self.damage.incoming_damage(change.damage / self.computed);
        self.value.combine_in_place(&change.modifier);
        self.compute();
    }
}

#[derive(Debug, Clone, Default, Reflect, Component)]
pub struct UnitVariablesChanges {
    unit: HashMap<IdRef<UnitVariableModel>, VariableValueChange>,
}

impl UnitVariablesChanges {
    /// Applies damage to the specified variable
    ///
    /// Positive damage values will reduce the variable's value, while negative
    /// damage values will increase it (healing).
    #[inline]
    pub fn damage(&mut self, id: IdRef<UnitVariableModel>, damage: f64) {
        let entry = self.unit.entry(id).or_default();
        entry.damage += damage;
    }

    /// Applies a modifier to the specified variable
    #[inline]
    pub fn modify(&mut self, id: IdRef<UnitVariableModel>, modifier: VariableValue) {
        let entry = self.unit.entry(id).or_default();
        entry.modifier += &modifier;
    }

    /// Applies both a modifier and damage to the specified variable
    #[inline]
    pub fn modify_and_damage(
        &mut self,
        id: IdRef<UnitVariableModel>,
        modifier: VariableValue,
        damage: f64,
    ) {
        let entry = self.unit.entry(id).or_default();
        entry.modifier += &modifier;
        entry.damage += damage;
    }
}

#[derive(Debug, Clone, Default, Reflect)]
struct VariableValueChange {
    /// Modifier to variable base params
    modifier: VariableValue,
    /// Flat damage to variable final value
    damage: f64,
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_damage() {
        let mut variable = VarData::new(
            VariableValue {
                base: 100.0,
                ..Default::default()
            },
            VariableValueDamage::ZERO,
            false,
        );

        assert_eq!(variable.computed, 100.0);

        variable.change(&VariableValueChange {
            modifier: Default::default(),
            damage: 30.0,
        });
        assert_eq!(variable.computed, 70.0);
    }

    #[test]
    fn test_damage_bonuses() {
        let mut variable = VarData::new(
            VariableValue {
                base: 25.0,
                bonus: 0.5,
                multiplier: 2.0,
                flat: 25.0,
            },
            VariableValueDamage::ZERO,
            false,
        );

        assert_eq!(variable.computed, 100.0);

        variable.change(&VariableValueChange {
            modifier: Default::default(),
            damage: 30.0,
        });
        assert_eq!(variable.computed, 70.0);
    }

    #[test]
    fn test_modify_damage() {
        let mut variable = VarData::new(
            VariableValue {
                base: 25.0,
                bonus: 0.5,
                multiplier: 2.0,
                flat: 25.0,
            },
            VariableValueDamage::ZERO,
            false,
        );

        assert_eq!(variable.computed, 100.0);

        variable.change(&VariableValueChange {
            modifier: VariableValue {
                flat: 30.0,
                ..Default::default()
            },
            damage: 30.0,
        });
        // took 30 damage from 100, so 70 left (70%), then added 30 flat to
        // base value, so 100 + 30 = 130, then applied damage multiplier of 0.7,
        // so 130 * 0.7 = 91
        assert_eq!(variable.computed, 91.0);
    }
}
