use std::collections::BTreeMap;
use crate::registries::unit_definition::UnitDefinitionModel;
use crate::types::sprite::SpriteId;
use bevy_reflect::Reflect;
use registry::registry::id::IdRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::registries::variable::UnitVariableModel;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(deny_unknown_fields)]
pub struct SpaceshipModel {
    pub sprite: SpriteId,
    pub model_size: f32,
    pub unit: IdRef<UnitDefinitionModel>,
    #[serde(default)]
    pub preset_variables: BTreeMap<IdRef<UnitVariableModel>, f64>,
}
