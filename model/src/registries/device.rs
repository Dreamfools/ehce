use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod tank_controller;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
pub struct DeviceModel {
    #[serde(flatten)]
    kind: DeviceKindModel,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
pub enum DeviceKindModel {
    TankController(tank_controller::TankControllerDeviceModel),
}
