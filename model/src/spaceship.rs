use crate::controllers::{ShipControllerModel, TankControllerModel};
use crate::sprite::{SpriteId, SpriteModel};
use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
pub struct SpaceshipModel {
    pub sprite: SpriteId,
    pub model_size: f32,
    pub controller: TankControllerModel,
}
