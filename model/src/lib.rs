use crate::settings::game_settings::GameSettings;
use crate::spaceship::SpaceshipModel;
use bevy_reflect::{Reflect, Type, Typed};
pub use common_model as common;
use registry::registry::entry::Entry;
use registry::registry::singleton::Singleton;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod controllers;
pub mod settings;
pub mod spaceship;
pub mod sprite;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
pub enum ModModel {
    // registries
    Spaceship(Entry<SpaceshipModel>),
    // settings
    GameSetting(Singleton<GameSettings>),
}

impl ModModel {
    pub fn required_singletons() -> Vec<&'static Type> {
        vec![GameSettings::type_info().ty()]
    }
}
