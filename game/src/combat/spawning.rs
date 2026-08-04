use crate::combat::CombatPostUpdate;
use crate::combat::device::DeviceOf;
use crate::combat::device::tank_controller::PhysicsTankController;
use crate::combat::init::CombatMarker;
use crate::combat::signals::UnitSignals;
use crate::combat::signals::inputs::PlayerBehavior;
use crate::combat::unit_variables::UnitVariables;
use crate::combat::visuals::camera::{
    CameraFollowSecondaryTarget, CameraFollowTarget, WorldWrapping, WorldWrappingCenter,
};
use avian2d::interpolation::TransformInterpolation;
use avian2d::prelude::{Collider, Mass, RigidBody};
use bevy::app::{App, Plugin};
use bevy::log::{info, warn};
use bevy::prelude::{
    Circle, Commands, EntityCommands, Message, Messages, Name, Res, ResMut, Sprite, Transform, Vec2,
};
use bevy::reflect::Reflect;
use mod_loading::mods::ModData;
use model::registries::device::{DeviceKindModel, DeviceModel};
use model::registries::ship_build::ShipBuildModel;
use model::registries::variable::UnitVariableMap;
use registry::registry::id::IdRef;
use registry::registry::reflect_registry::ReflectRegistry;
use utils::map::HashSet;

pub struct SpawningPlugin;

impl Plugin for SpawningPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(CombatPostUpdate, sys_spawn_spaceships)
            .add_message::<SpawnSpaceshipMessage>();
    }
}

#[derive(Debug, Message, Reflect)]
pub struct SpawnSpaceshipMessage {
    pub id: IdRef<ShipBuildModel>,
    pub position: Vec2,
    pub extra_devices: Vec<IdRef<DeviceModel>>,
}

fn sys_spawn_spaceships(
    mod_data: Res<ModData>,
    mut commands: Commands,
    mut messages: ResMut<Messages<SpawnSpaceshipMessage>>,
) {
    for msg in messages.drain() {
        spawn_spaceship(&mod_data.registry, commands.reborrow(), msg);
    }
}

fn spawn_spaceship(reg: &ReflectRegistry, mut commands: Commands, msg: SpawnSpaceshipMessage) {
    info!(
        "Spawning spaceship with id {} at position {:?}",
        msg.id, msg.position
    );

    let ship_build = &reg[msg.id];
    let ship = &reg[ship_build.ship];

    let circle = Circle::new(ship.model_size / 2.0);

    let mut sprite = Sprite::from_image(reg[ship.sprite].clone());
    sprite.custom_size = Some(Vec2::splat(1.0));

    let unit_def = &reg[ship.unit];

    let mut vars = UnitVariableMap::default();
    for (id, value) in unit_def
        .preset_variables
        .iter()
        .chain(ship.preset_variables.iter())
    {
        vars.insert(*id, *value);
    }

    let mut comp_devices = Vec::new();

    for comp in &ship_build.components {
        let comp_data = &reg[comp.id];
        let comp_stats = &reg[comp_data.stats];

        if let Some(device) = comp_data.device {
            comp_devices.push(device);
        }

        for (id, value) in &comp_stats.variables {
            *vars.entry(*id).or_insert_with(|| reg[id].default_value) += value;
        }
    }

    let mut entity = commands.spawn((
        Name::new(msg.id.to_string()),
        CombatMarker,
        RigidBody::Dynamic,
        Collider::from(circle),
        TransformInterpolation,
        Transform::from_xyz(msg.position.x, msg.position.y, 0.0),
        sprite,
        UnitSignals::bundle(),
        UnitVariables::new(reg, &vars),
        Mass(1.0),
        CameraFollowSecondaryTarget,
        WorldWrapping,
    ));

    // TODO: store these active devices list in a component?
    let mut active_devices = Default::default();
    for device in unit_def
        .builtin_devices
        .iter()
        .chain(comp_devices.iter())
        .chain(msg.extra_devices.iter())
    {
        spawn_device(reg, entity.reborrow(), &mut active_devices, device);
    }
}

fn spawn_device(
    reg: &ReflectRegistry,
    mut entity: EntityCommands,
    active_devices: &mut HashSet<IdRef<DeviceModel>>,
    device_id: &IdRef<DeviceModel>,
) {
    let device = &reg[device_id];
    if !active_devices.insert(*device_id) && device.unique {
        warn!(
            "Device {} is unique but was already active on the spaceship, skipping",
            device_id
        );
        return;
    }

    match &device.kind {
        DeviceKindModel::TankController(tank) => {
            entity.with_related::<DeviceOf>(PhysicsTankController::from_device(tank));
        }
        DeviceKindModel::PlayerInputs(_) => {
            entity.insert((CameraFollowTarget, WorldWrappingCenter));
            entity.with_related::<DeviceOf>(PlayerBehavior::Directional);
        }
    }
}
