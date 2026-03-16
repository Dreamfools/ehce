use crate::types::formula::formula_context::UnitFormulaModel;
use crate::registries::variable::UnitVariableModel;
use bevy_reflect::Reflect;
use registry::registry::id::IdRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
pub struct UnitDefinitionModel {
    pub weight: UnitFormulaModel,
    pub preset_variables: BTreeMap<IdRef<UnitVariableModel>, f64>,
}
