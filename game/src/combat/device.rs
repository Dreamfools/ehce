use crate::combat::CombatPostUpdate;
use bevy::app::{App, Plugin};
use bevy::prelude::{Component, Reflect};

pub mod tank_controller;

#[derive(Debug, Clone, Reflect, Component)]
pub struct ControllerMaxSpeed {
    pub max_speed: f32,
    pub max_angular_speed: f32,
}

pub struct DevicePlugin;

impl Plugin for DevicePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(CombatPostUpdate, tank_controller::tank_controller_update);
    }
}
