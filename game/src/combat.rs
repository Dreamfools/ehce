use bevy::app::{App, PluginGroup as _};
use bevy::prelude::Plugin;

pub mod controller;
pub mod spawning;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            // Avian2d physics
            avian2d::PhysicsPlugins::default()
                .with_length_unit(1.0)
                .set(avian2d::interpolation::PhysicsInterpolationPlugin::interpolate_all()),
            // Controllers
            controller::ShipControllerPlugin,
        ));
    }
}
