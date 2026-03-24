use crate::combat::signals::{SignalId, SignalValue, UnitSignals};
use crate::ecs_tools::component_invariants;
use bevy::math::{Vec3, Vec3Swizzles as _};
use bevy::prelude::{ButtonInput, Component, GlobalTransform, KeyCode, Query, Reflect, Res};

#[derive(Debug, Clone, Reflect, Component)]
pub enum PlayerBehavior {
    Tank,
    Directional,
}

component_invariants!(PlayerBehavior: UnitSignals);

pub fn update_player_behavior(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    q: Query<(&GlobalTransform, &PlayerBehavior, &mut UnitSignals)>,
) {
    for (transform, behavior, mut input) in q {
        let mut facing = SignalValue::Off;
        let mut movement = SignalValue::Off;
        match behavior {
            PlayerBehavior::Tank => {
                let thrust = if keyboard_input.pressed(KeyCode::KeyW) {
                    1.0
                } else if keyboard_input.pressed(KeyCode::KeyS) {
                    -1.0
                } else {
                    0.0
                };
                movement = SignalValue::Scalar(thrust);
                let mut facing_val = 0.0;
                if keyboard_input.pressed(KeyCode::KeyA) {
                    facing_val += 1.0;
                }
                if keyboard_input.pressed(KeyCode::KeyD) {
                    facing_val -= 1.0;
                };
                if facing_val != 0.0 {
                    facing = SignalValue::Scalar(facing_val);
                }
            }
            PlayerBehavior::Directional => {
                let mut x = 0.0;
                let mut y = 0.0;
                if keyboard_input.pressed(KeyCode::KeyW) {
                    y += 1.0;
                };
                if keyboard_input.pressed(KeyCode::KeyS) {
                    y -= 1.0;
                };
                if keyboard_input.pressed(KeyCode::KeyA) {
                    x -= 1.0;
                };
                if keyboard_input.pressed(KeyCode::KeyD) {
                    x += 1.0;
                };

                if x != 0.0 || y != 0.0 {
                    let direction = (transform.rotation().inverse() * Vec3::new(x, y, 0.0))
                        .normalize_or_zero()
                        .xy();
                    facing = SignalValue::Vector(direction);
                    movement = SignalValue::Scalar(1.0);
                }
            }
        }

        input.set(SignalId::Facing, facing);
        input.set(SignalId::Movement, movement);
    }
}
