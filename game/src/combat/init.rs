use crate::combat::settings::CombatSettings;
use crate::combat::spawning::SpawnSpaceshipMessage;
use crate::combat::visuals::camera::{CombatBackground, CombatCamera};
use crate::combat::visuals::space_background::BackgroundMaterial;
use crate::state::GameState;
use avian2d::parry::glamx::Vec3;
use bevy::app::{App, FixedUpdate, Plugin};
use bevy::camera::{Camera2d, OrthographicProjection, Projection, ScalingMode};
use bevy::input::common_conditions::input_pressed;
use bevy::math::{Vec2, Vec4};
use bevy::mesh::{Mesh, Mesh2d};
use bevy::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs as _, KeyCode, MeshMaterial2d, Message,
    MessageWriter, Messages, OnEnter, Rectangle, Reflect, ResMut, Transform, With, World,
};
use bevy::sprite_render::Material2dPlugin;
use bevy_asset::Assets;
use model::registries::ship_build::ShipBuildModel;
use registry::registry::id::{IdRef, RawId};

pub struct InitPlugin;

impl Plugin for InitPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<BackgroundMaterial>::default())
            .add_systems(OnEnter(GameState::Gameplay), sys_init_battlefield)
            .add_systems(
                FixedUpdate,
                (
                    (sys_handle_combat_clear, sys_handle_combat_init).chain(),
                    sys_init_battlefield.run_if(input_pressed(KeyCode::F5)),
                ),
            )
            .add_message::<CombatInitMessage>()
            .add_message::<CombatClearMessage>();
    }
}

#[derive(Debug, Clone, Reflect, Component)]
pub struct CombatMarker;

#[derive(Debug, Message, Reflect)]
pub struct CombatInitMessage {}

#[derive(Debug, Message, Reflect)]
pub struct CombatClearMessage {}
fn sys_init_battlefield(mut messages: ResMut<Messages<CombatInitMessage>>) {
    messages.write(CombatInitMessage {});
}

fn sys_handle_combat_init(
    mut messages: ResMut<Messages<CombatInitMessage>>,
    mut commands: Commands,
    mut spawn_ships: MessageWriter<SpawnSpaceshipMessage>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BackgroundMaterial>>,
) {
    let Some(_msg) = messages.drain().last() else {
        return;
    };

    commands.insert_resource(CombatSettings::default());

    // Spawn a camera.
    commands
        .spawn((
            CombatMarker,
            CombatCamera::default(),
            Camera2d,
            Projection::Orthographic(OrthographicProjection {
                near: -1e9,
                far: 1e9,
                scaling_mode: ScalingMode::AutoMax {
                    max_height: 64.0,
                    max_width: 64.0,
                },
                ..OrthographicProjection::default_2d()
            }),
        ))
        .with_child((
            CombatMarker,
            CombatBackground,
            Mesh2d(meshes.add(Rectangle::default())),
            MeshMaterial2d(materials.add(BackgroundMaterial {
                transform: Vec4::ZERO,
            })),
            Transform::default().with_scale(Vec3::splat(64.)),
        ));

    spawn_ships.write(SpawnSpaceshipMessage {
        id: IdRef::<ShipBuildModel>::new(RawId::new("base:scout")),
        position: Vec2::new(10.0, 0.0),
        extra_devices: vec![IdRef::new(RawId::new("core:player_inputs"))],
    });

    spawn_ships.write(SpawnSpaceshipMessage {
        id: IdRef::<ShipBuildModel>::new(RawId::new("base:scout")),
        position: Vec2::new(0.0, 0.0),
        extra_devices: vec![],
    });
}

/// Clear all combat entities if a [CombatClearMessage] is received or if a
/// [CombatInitMessage] is received
///
/// Also consumes all [CombatClearMessage]s but leaves [CombatInitMessage]s in
/// the queue so that they can be handled by [sys_handle_combat_init]
fn sys_handle_combat_clear(w: &mut World) {
    let mut want_clear;

    // resources are inserted by plugin
    #[cfg_attr(bevy_lint, allow(bevy::panicking_methods))]
    let mut clear_messages = w.resource_mut::<Messages<CombatClearMessage>>();
    want_clear = !clear_messages.is_empty();
    clear_messages.clear();

    #[cfg_attr(bevy_lint, allow(bevy::panicking_methods))]
    let init_messages = w.resource::<Messages<CombatInitMessage>>();
    if !init_messages.is_empty() {
        want_clear = true;
    }

    if !want_clear {
        return;
    }

    let ents: Vec<_> = w
        .query_filtered::<Entity, With<CombatMarker>>()
        .query(w)
        .iter()
        .collect();

    for ent in ents {
        w.despawn(ent);
    }
}
