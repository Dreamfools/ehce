use crate::types::formula::formula_context::UnitFormulaModel;
use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(deny_unknown_fields)]
pub struct TankControllerDeviceModel {
    pub acceleration_force: UnitFormulaModel,
    pub braking_force: UnitFormulaModel,
    pub turn_torque: UnitFormulaModel,
    pub max_speed: UnitFormulaModel,
    pub max_angular_speed: UnitFormulaModel,
}
