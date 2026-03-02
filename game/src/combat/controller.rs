use crate::state::GameState;
use bevy::app::{App, FixedUpdate, Plugin};
use bevy::prelude::{Component, IntoScheduleConfigs, Reflect, in_state};

pub mod behavior;
pub mod inputs;
pub mod tank_controller;

#[derive(Debug, Clone, Reflect, Component)]
pub struct ControllerMaxSpeed {
    pub max_speed: f32,
    pub max_angular_speed: f32,
}

pub struct ShipControllerPlugin;

impl Plugin for ShipControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                behavior::player_behavior::update_player_behavior,
                tank_controller::tank_controller_update,
            )
                .run_if(in_state(GameState::Gameplay)),
        );
    }
}
