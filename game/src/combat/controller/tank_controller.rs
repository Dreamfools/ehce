use crate::combat::controller::inputs::ControllerInputs;
use crate::ecs_tools::component_invariants;
use avian2d::prelude::{AngularDamping, AngularVelocity, ComputedMass, LinearVelocity, RigidBody};
use bevy::math::Vec2;
use bevy::prelude::{Component, Query, Reflect, Res, Transform};
use bevy::time::{Fixed, Time};
use model::controllers::TankControllerModel;

#[derive(Debug, Clone, Reflect, Component)]
pub struct PhysicsTankController {
    pub acceleration_force: f32,
    pub deceleration_force: f32,
    pub braking_force: f32,
    pub turn_torgue: f32,
    pub max_speed: f32,
    pub max_angular_speed: f32,
}

component_invariants!(PhysicsTankController : RigidBody, ControllerInputs);

impl PhysicsTankController {
    pub fn from_model(model: &TankControllerModel) -> PhysicsTankController {
        Self {
            acceleration_force: model.acceleration_force,
            deceleration_force: model.deceleration_force,
            braking_force: model.braking_force,
            turn_torgue: model.turn_torgue,
            max_speed: model.max_speed,
            max_angular_speed: model.max_angular_speed,
        }
    }
}

pub fn tank_controller_update(
    fixed_time: Res<Time<Fixed>>,
    q: Query<(
        &Transform,
        &ControllerInputs,
        &PhysicsTankController,
        &ComputedMass,
        &RigidBody,
        &mut LinearVelocity,
        &mut AngularVelocity,
        Option<&AngularDamping>,
    )>,
) {
    let dt = fixed_time.delta_secs();
    for (
        transform,
        inputs,
        controller,
        mass,
        rb,
        mut linear_velocity,
        mut angular_velocity,
        angular_damping,
    ) in q
    {
        if rb.is_static() || !mass.inverse().is_finite() {
            continue;
        }

        let torgue = controller.turn_torgue * mass.inverse();

        let ship_direction = transform.right().truncate();

        let mut want_turn = 0.0;

        if inputs.turn != 0.0 {
            want_turn = inputs.turn.clamp(-1.0, 1.0);
        } else {
            let want_direction = inputs.direction.normalize_or_zero();
            if want_direction != Vec2::ZERO {
                let turn_angle = ship_direction.angle_to(want_direction);
                if angular_velocity.0 != 0.0 && angular_velocity.0.signum() != turn_angle.signum() {
                    // rotating in opposite direction, start braking
                    want_turn = 0.0;
                } else {
                    let distance = distance_traveled(
                        angular_velocity.0,
                        torgue + angular_damping.map_or(0.0, |d| d.0),
                    );
                    if (distance - turn_angle.abs()) > std::f32::consts::PI / 180.0 {
                        // start braking
                        want_turn = 0.0;
                    } else {
                        // continue accelerating in the same direction

                        // how much do we need to turn to get to the desired direction in one tick?
                        let turn_to_achieve = turn_angle / (torgue * dt);

                        want_turn = turn_to_achieve.clamp(-1.0, 1.0);
                    }
                }
            }
        }

        if want_turn != 0.0 {
            if angular_velocity.0.signum() == want_turn.signum()
                && angular_velocity.abs() >= controller.max_angular_speed
            {
                // already at max speed
            } else {
                angular_velocity.0 += want_turn * torgue * dt;
            }
        } else if angular_velocity.0 != 0.0 {
            // damp rotation when not turning
            let damping = torgue * dt;
            if angular_velocity.0.abs() <= damping {
                angular_velocity.0 = 0.0;
            } else {
                angular_velocity.0 -= angular_velocity.0.signum() * damping;
            }
        }

        if inputs.throttle != 0.0 {
            let acceleration = controller.acceleration_force
                * mass.inverse()
                * inputs.throttle.clamp(-1.0, 1.0)
                * dt
                * ship_direction;
            let new_velocity = linear_velocity.0 + acceleration;
            let velocity_l2 = linear_velocity.length_squared();
            let new_velocity_l2 = new_velocity.length_squared();
            if new_velocity_l2 > controller.max_speed * controller.max_speed
                && new_velocity_l2 > velocity_l2
            {
                // When exceeding the speed limit, add some speed in the direction but reduce overall velocity to maintain the same speed
                let acceleration_factor = (velocity_l2 / new_velocity_l2).sqrt();
                linear_velocity.0 = new_velocity * acceleration_factor;
            } else {
                linear_velocity.0 = new_velocity;
            }
        }

        if inputs.brake != 0.0 {
            let brake_force =
                controller.braking_force * mass.inverse() * inputs.brake.clamp(0.0, 1.0) * dt;
            if linear_velocity.length_squared() <= brake_force * brake_force {
                linear_velocity.0 = Vec2::ZERO;
            } else {
                let decel = linear_velocity.normalize() * brake_force;
                linear_velocity.0 -= decel;
            }
        }
    }
}

/// Distance traveled until stop when decelerating from `speed` with `deceleration`.
fn distance_traveled(speed: f32, deceleration: f32) -> f32 {
    speed * speed / 2.0 / deceleration
}
