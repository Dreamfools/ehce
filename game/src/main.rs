#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]

//! This example showcases how `Transform` interpolation or extrapolation can be used
//! to make movement appear smooth at fixed timesteps.
//!
//! To produce consistent, frame rate independent behavior, physics by default runs
//! in the `FixedPostUpdate` schedule with a fixed timestep, meaning that the time between
//! physics ticks remains constant. On some frames, physics can either not run at all or run
//! more than once to catch up to real time. This can lead to visible stutter for movement.
//!
//! `Transform` interpolation resolves this issue by updating `Transform` at every frame in between
//! physics ticks to smooth out the visual result. The interpolation is done from the previous position
//! to the current physics position, which keeps movement smooth, but has the downside of making movement
//! feel slightly delayed as the rendered result lags slightly behind the true positions.
//!
//! `Transform` extrapolation works similarly, but instead of using the previous positions, it predicts
//! the next positions based on velocity. This makes movement feel much more responsive, but can cause
//! jumpy results when the prediction is wrong, such as when the velocity of an object is suddenly altered.

pub mod state;

pub mod combat;
pub mod ecs_tools;

use crate::combat::CombatPlugin;
use crate::combat::signals::inputs::PlayerBehavior;
use crate::combat::spawning::SpawnSpaceshipMessage;
use crate::state::GameState;
use bevy::camera::ScalingMode;
use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy::{input::common_conditions::input_pressed, prelude::*};
use mod_asset_source::MODS_FOLDER;
use mod_loading::json5_asset_plugin::Json5AssetPlugin;
use mod_loading::loading::{CustomAssetReaderPlugin, DatabaseAsset, load_last_mod};
use mod_loading::mods::{ModLoadErrorMessage, ModLoadedMessage, ModPlugin, ModState};
use model::registries::spaceship::SpaceshipModel;
use registry::registry::id::{IdRef, RawId};

fn main() -> AppExit {
    let mut app = App::new();

    app.edit_schedule(Update, |schedule| {
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Error,
            ..default()
        });
    });

    // Interpolation and extrapolation functionality is enabled by the `PhysicsInterpolationPlugin`.
    // It is included in the `PhysicsPlugins` by default.
    app.add_plugins((
        CustomAssetReaderPlugin,
        DefaultPlugins.set(AssetPlugin {
            mode: AssetMode::Unprocessed,
            file_path: MODS_FOLDER.to_owned(),
            processed_file_path: "tmp".to_string(),
            ..Default::default()
        }),
        Json5AssetPlugin::<DatabaseAsset>::new(&["json", "json5"]),
        ModPlugin,
        CombatPlugin,
    ));

    app.insert_state(GameState::default());

    // 60hz fixed timestep.
    app.insert_resource(Time::from_hz(60.0));

    app.add_systems(OnEnter(GameState::Init), load_last_mod)
        .add_systems(PostUpdate, (init_tick).run_if(in_state(GameState::Init)))
        .add_systems(OnEnter(GameState::Gameplay), (setup_scene, setup_ships))
        .add_systems(PostUpdate, handle_mod_loaded_error_message);

    // Setup the scene and UI, and update text in `Update`.
    app.add_systems(
        Update,
        (
            // Reset the scene when the 'R' key is pressed.
            reset_balls.run_if(input_pressed(KeyCode::KeyR)),
        ),
    );

    // Run the app.
    app.run()
}

fn init_tick(
    mut errors: MessageReader<ModLoadErrorMessage>,
    mut loaded: ResMut<Messages<ModLoadedMessage>>,
    mut state: ResMut<NextState<GameState>>,
    mut mod_state: ResMut<NextState<ModState>>,
    mut commands: Commands,
) {
    if errors.read().next().is_some() {
        state.set(GameState::Error);
        mod_state.set(ModState::None);
        warn!("Got a mod loading error during initialization, switching to error state");
        return;
    }

    if let Some(data) = loaded.drain().last() {
        info!("Mod is loaded, switching to combat state");
        let mod_data = data.0;

        commands.insert_resource(mod_data);
        mod_state.set(ModState::Ready);
        state.set(GameState::Gameplay);
    }
}

#[derive(Reflect, Component)]
struct Ball;

fn setup_scene(mut commands: Commands) {
    // Spawn a camera.
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            near: -1e9,
            far: 1e9,
            scaling_mode: ScalingMode::AutoMax {
                max_width: 64.0,
                max_height: 64.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

fn setup_ships(mut spawn_ships: MessageWriter<SpawnSpaceshipMessage>) {
    spawn_ships.write(SpawnSpaceshipMessage {
        id: IdRef::<SpaceshipModel>::new(RawId::new("base:scout")),
        position: Default::default(),
    });
}

/// Despawns all balls and respawns them.
fn reset_balls(mut commands: Commands, query: Query<Entity, With<PlayerBehavior>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }

    commands.run_system_cached(setup_ships);
}

fn handle_mod_loaded_error_message(mut errs: MessageReader<ModLoadErrorMessage>) {
    for msg in errs.read() {
        error!("Something gone wrong.\n{:?}", msg.0);
    }
}
