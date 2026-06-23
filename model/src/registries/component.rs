use crate::registries::component_stats::ComponentStatsModel;
use crate::registries::device::DeviceModel;
use bevy_reflect::Reflect;
use registry::registry::id::IdRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(deny_unknown_fields)]
pub struct ComponentModel {
    pub name: String,
    pub stats: IdRef<ComponentStatsModel>,
    #[serde(default)]
    pub device: Option<IdRef<DeviceModel>>,
}
