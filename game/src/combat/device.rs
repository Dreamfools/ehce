use crate::combat::CombatPostUpdate;
use bevy::app::{App, Plugin};
use bevy::prelude::{Component, Entity};

pub mod tank_controller;

pub struct DevicePlugin;

impl Plugin for DevicePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(CombatPostUpdate, tank_controller::tank_controller_update);
    }
}

#[derive(Component, Debug)]
#[relationship(relationship_target = AttachedDevices)]
pub struct DeviceOf(Entity);

impl DeviceOf {
    #[must_use]
    pub fn parent(&self) -> Entity {
        self.0
    }
}

#[derive(Component, Debug)]
#[relationship_target(relationship = DeviceOf, linked_spawn)]
pub struct AttachedDevices(Vec<Entity>);
