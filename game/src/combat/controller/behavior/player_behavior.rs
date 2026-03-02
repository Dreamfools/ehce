use crate::combat::controller::inputs::ControllerInputs;
use crate::ecs_tools::component_invariants;
use bevy::input::ButtonInput;
use bevy::log::info;
use bevy::math::Vec2;
use bevy::prelude::{Component, KeyCode, Query, Res, With};
use bevy::reflect::Reflect;

#[derive(Debug, Clone, Reflect, Component)]
pub enum PlayerBehavior {
    Tank,
    Directional,
}

component_invariants!(PlayerBehavior: ControllerInputs);

pub fn update_player_behavior(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    q: Query<(&PlayerBehavior, &mut ControllerInputs)>,
) {
    for (behavior, mut input) in q {
        match behavior {
            PlayerBehavior::Tank => {
                input.throttle = if keyboard_input.pressed(KeyCode::KeyW) {
                    1.0
                } else if keyboard_input.pressed(KeyCode::KeyS) {
                    -1.0
                } else {
                    0.0
                };
                input.turn = if keyboard_input.pressed(KeyCode::KeyA) {
                    1.0
                } else if keyboard_input.pressed(KeyCode::KeyD) {
                    -1.0
                } else {
                    0.0
                };
            }
            PlayerBehavior::Directional => {
                let up = if keyboard_input.pressed(KeyCode::KeyW) {
                    1.0
                } else {
                    0.0
                };
                let down = if keyboard_input.pressed(KeyCode::KeyS) {
                    -1.0
                } else {
                    0.0
                };
                let left = if keyboard_input.pressed(KeyCode::KeyA) {
                    -1.0
                } else {
                    0.0
                };
                let right = if keyboard_input.pressed(KeyCode::KeyD) {
                    1.0
                } else {
                    0.0
                };
                input.direction = Vec2::new(left + right, up + down).normalize_or_zero();
                input.throttle = (input.direction != Vec2::ZERO)
                    .then_some(1.0)
                    .unwrap_or(0.0);
            }
        }
    }
}
