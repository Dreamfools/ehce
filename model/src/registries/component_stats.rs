use crate::registries::variable::UnitVariableMap;
use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(deny_unknown_fields)]
pub struct ComponentStatsModel {
    pub variables: UnitVariableMap,
}
