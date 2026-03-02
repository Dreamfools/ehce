use bevy::math::Vec2;
use bevy::prelude::{Component, Reflect};

#[derive(Debug, Clone, Default, Reflect, Component)]
pub struct ControllerInputs {
    /// Direction the player wants to move in, absolute in world space
    pub direction: Vec2,
    /// Rotation input, -1.0 to 1.0, where -1.0 is full left and 1.0 is full right
    pub turn: f32,
    /// Throttle input, -1.0 to 1.0, where -1.0 is full reverse and 1.0 is full forward
    pub throttle: f32,
    /// Braking input, 0.0 to 1.0, where 0.0 is no braking and 1.0 is full braking
    pub brake: f32,
}
