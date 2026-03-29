use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(deny_unknown_fields)]
pub struct TankControllerDeviceModel {
    pub acceleration_force: f32,
    pub braking_force: f32,
    pub turn_torgue: f32,
    pub max_speed: f32,
    pub max_angular_speed: f32,
}
