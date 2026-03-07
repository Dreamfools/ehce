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

mod loading;
mod mods;
mod state;

mod combat;
mod ecs_tools;

use crate::combat::CombatPlugin;
use crate::combat::controller::behavior::player_behavior::PlayerBehavior;
use crate::combat::controller::inputs::ControllerInputs;
use crate::combat::controller::tank_controller::PhysicsTankController;
use crate::loading::json5_asset_plugin::Json5AssetPlugin;
use crate::loading::{CustomAssetReaderPlugin, DatabaseAsset, load_last_mod};
use crate::mods::{ModData, ModLoadErrorMessage, ModLoadedMessage, ModPlugin, ModState};
use crate::state::GameState;
use avian2d::prelude::*;
use bevy::camera::ScalingMode;
use bevy::{
    color::palettes::{
        css::WHITE,
        tailwind::{CYAN_400, LIME_400, RED_400},
    },
    input::common_conditions::input_pressed,
    prelude::*,
};
use inline_tweak::tweak;
use mod_asset_source::MODS_FOLDER;
use model::spaceship::SpaceshipModel;
use registry::registry::id::{IdRef, RawId};

fn main() -> AppExit {
    let mut app = App::new();

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
    app.insert_resource(Gravity(Vec2::ZERO));

    app.add_systems(OnEnter(GameState::Init), load_last_mod)
        .add_systems(PostUpdate, (init_tick).run_if(in_state(GameState::Init)))
        .add_systems(
            OnEnter(GameState::Gameplay),
            (setup_scene, setup_ships, setup_text),
        )
        .add_systems(Update, handle_mod_loaded_error_message);

    // Setup the scene and UI, and update text in `Update`.
    app.add_systems(
        Update,
        (
            change_timestep,
            update_timestep_text,
            // Reset the scene when the 'R' key is pressed.
            reset_balls.run_if(input_pressed(KeyCode::KeyR)),
        ),
    )
    .add_systems(FixedUpdate, move_balls);

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

fn setup_ships(
    mut commands: Commands,
    _materials: ResMut<Assets<ColorMaterial>>,
    _meshes: ResMut<Assets<Mesh>>,
    mod_data: Res<ModData>,
) {
    let scout = &mod_data.registry[IdRef::<SpaceshipModel>::new(RawId::new("scout"))];
    let circle = Circle::new(30.0);

    let mut sprite = Sprite::from_image(mod_data.registry[scout.sprite].clone());
    sprite.custom_size = Some(Vec2::splat(1.0));

    // This entity uses transform interpolation.
    commands.spawn((
        Name::new("Interpolation"),
        RigidBody::Dynamic,
        Collider::from(circle),
        TransformInterpolation,
        Transform::from_xyz(0.0, 0.0, 0.0),
        sprite,
        PlayerBehavior::Directional,
        ControllerInputs::default(),
        PhysicsTankController::from_model(&scout.controller),
        Mass(1.0),
        // Mesh2d(mesh.clone()),
        // MeshMaterial2d(materials.add(Color::from(CYAN_400)).clone()),
    ));
}

/// Despawns all balls and respawns them.
fn reset_balls(mut commands: Commands, query: Query<Entity, With<PlayerBehavior>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }

    commands.run_system_cached(setup_ships);
}

#[derive(Reflect, Component)]
struct TimestepText;

fn setup_text(mut commands: Commands) {
    let font = TextFont {
        font_size: 20.0,
        ..default()
    };

    commands
        .spawn((
            Text::new("Fixed Hz: "),
            TextColor::from(WHITE),
            font.clone(),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
        ))
        .with_child((TimestepText, TextSpan::default()));

    commands.spawn((
        Text::new("Change Timestep With Up/Down Arrow\nPress R to reset"),
        TextColor::from(WHITE),
        TextLayout::new_with_justify(Justify::Right),
        font.clone(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
    ));

    commands.spawn((
        Text::new("Interpolation"),
        TextColor::from(CYAN_400),
        font.clone(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(50.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    commands.spawn((
        Text::new("Extrapolation"),
        TextColor::from(LIME_400),
        font.clone(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(75.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    commands.spawn((
        Text::new("No Interpolation"),
        TextColor::from(RED_400),
        font.clone(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(100.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

/// Changes the timestep of the simulation when the up or down arrow keys are pressed.
fn change_timestep(mut time: ResMut<Time<Fixed>>, keyboard_input: Res<ButtonInput<KeyCode>>) {
    if keyboard_input.pressed(KeyCode::ArrowUp) {
        let new_timestep = (time.delta_secs_f64() * 0.975).max(1.0 / 255.0);
        time.set_timestep_seconds(new_timestep);
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        let new_timestep = (time.delta_secs_f64() * 1.025).min(1.0 / 5.0);
        time.set_timestep_seconds(new_timestep);
    }
}

/// Updates the text with the current timestep.
fn update_timestep_text(
    mut text: Single<&mut TextSpan, With<TimestepText>>,
    time: Res<Time<Fixed>>,
) {
    let timestep = time.timestep().as_secs_f32().recip();
    text.0 = format!("{timestep:.2}");
}

fn move_balls(mut query: Query<&mut Transform, With<Ball>>) {
    for mut transform in &mut query {
        transform.translation.x += tweak!(1.0);
    }
}

fn handle_mod_loaded_error_message(mut errs: MessageReader<ModLoadErrorMessage>) {
    for msg in errs.read() {
        error!("Something gone wrong.\n{:?}", msg.0);
    }
}
