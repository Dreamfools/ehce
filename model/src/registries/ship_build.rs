use crate::registries::component::ComponentModel;
use crate::registries::spaceship::SpaceshipModel;
use bevy_reflect::Reflect;
use registry::registry::id::IdRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(deny_unknown_fields)]
pub struct ShipBuildModel {
    pub ship: IdRef<SpaceshipModel>,
    pub components: Vec<InstalledComponentModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(deny_unknown_fields)]
pub struct InstalledComponentModel {
    pub id: IdRef<ComponentModel>,
}
