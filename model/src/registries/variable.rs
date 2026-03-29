use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(deny_unknown_fields)]
pub struct UnitVariableModel {
    /// The default value of the variable
    pub default_value: f64,
    /// Whether the variable is read-only (cannot be modified after being initialized)
    pub readonly: bool,
}
