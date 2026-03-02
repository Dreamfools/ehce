use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
pub struct TankControllerModel {
    pub acceleration_force: f32,
    pub deceleration_force: f32,
    pub braking_force: f32,
    pub turn_torgue: f32,
    pub max_speed: f32,
    pub max_angular_speed: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[repr(C)]
// #[serde(tag = "type", rename_all = "snake_case")]
pub enum ShipControllerModel {
    Tank(TankControllerModel),
}
