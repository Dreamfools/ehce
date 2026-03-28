use crate::types::signal::ActivationMode;
use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod player_inputs;
pub mod tank_controller;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(deny_unknown_fields)]
pub struct DeviceModel {
    #[serde(flatten)]
    pub kind: DeviceKindModel,
    pub activation: ActivationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKindModel {
    PlayerInputs(player_inputs::PlayerInputsDeviceModel),
    TankController(tank_controller::TankControllerDeviceModel),
}
