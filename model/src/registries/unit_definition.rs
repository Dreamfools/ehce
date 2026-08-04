use crate::registries::device::DeviceModel;
use crate::registries::variable::UnitVariableMap;
use crate::types::formula::formula_context::UnitFormulaModel;
use bevy_reflect::Reflect;
use registry::registry::id::IdRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(deny_unknown_fields)]
pub struct UnitDefinitionModel {
    pub weight: UnitFormulaModel,
    pub preset_variables: UnitVariableMap,
    pub builtin_devices: Vec<IdRef<DeviceModel>>,
}
