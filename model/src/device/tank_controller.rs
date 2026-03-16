use crate::formula::formula_context::UnitFormulaModel;
use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
pub struct TankControllerDeviceModel {
    pub acceleration_force: UnitFormulaModel,
    pub deceleration_force: UnitFormulaModel,
    pub braking_force: UnitFormulaModel,
    pub turn_torgue: UnitFormulaModel,
    pub max_speed: UnitFormulaModel,
    pub max_angular_speed: UnitFormulaModel,
}
