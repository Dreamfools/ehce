use crate::sprite::SpriteId;
use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
pub struct SpaceshipModel {
    pub sprite: SpriteId,
    pub model_size: f32,
}
