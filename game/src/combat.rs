use crate::state::GameState;
use avian2d::prelude::Gravity;
use bevy::app::{App, FixedMainScheduleOrder, FixedUpdate, PluginGroup as _};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::math::Vec2;
use bevy::prelude::{IntoScheduleConfigs as _, Plugin, World, in_state};

pub mod device;
pub mod signals;
pub mod spawning;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        #[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
        struct CombatFixedUpdateLoop;

        app.init_schedule(CombatFixedUpdateLoop);

        app.init_schedule(CombatInputs);
        app.init_schedule(CombatUpdate);
        app.init_schedule(CombatPostUpdate);
        app.init_schedule(CombatPhysicsUpdate);

        #[cfg_attr(bevy_lint, allow(bevy::panicking_methods))]
        // fixed schedule should exist for combat to work
        app.world_mut()
            .resource_mut::<FixedMainScheduleOrder>()
            .insert_after(FixedUpdate, CombatFixedUpdateLoop);

        app.add_systems(
            CombatFixedUpdateLoop,
            update_combat_schedules.run_if(in_state(GameState::Gameplay)),
        );

        app.insert_resource(Gravity(Vec2::ZERO));

        app.add_plugins((
            // Avian2d physics
            avian2d::PhysicsPlugins::new(CombatPhysicsUpdate)
                .with_length_unit(1.0)
                .set(avian2d::interpolation::PhysicsInterpolationPlugin::interpolate_all()),
            // Plugins
            device::DevicePlugin,
            spawning::SpawningPlugin,
            signals::SignalsPlugin,
        ));
    }
}

#[cfg_attr(bevy_lint, allow(bevy::panicking_methods))] // schedules are registered and exist
fn update_combat_schedules(world: &mut World) {
    world.run_schedule(CombatPhysicsUpdate);
    world.run_schedule(CombatInputs);
    world.run_schedule(CombatUpdate);
    world.run_schedule(CombatPostUpdate);
}

/// Schedule for processing combat physics
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct CombatPhysicsUpdate;
/// Schedule for collecting inputs for combat
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct CombatInputs;
/// Schedule for main combat logic processing and emitting events to process in the post update stage
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct CombatUpdate;
/// Schedule for processing events emitted in the combat update stage, and doing any necessary updates
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct CombatPostUpdate;
