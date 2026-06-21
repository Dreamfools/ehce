use crate::registries::variable::UnitVariableModel;
use bevy_reflect::Reflect;
use registry::registry::id::IdRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(deny_unknown_fields)]
pub struct ComponentStatsModel {
    pub variables: BTreeMap<IdRef<UnitVariableModel>, f64>,
}
