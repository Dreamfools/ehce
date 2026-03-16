use registries::device::DeviceModel;
use crate::settings::game_settings::GameSettings;
use registries::spaceship::SpaceshipModel;
use registries::unit_definition::UnitDefinitionModel;
use registries::variable::UnitVariableModel;
use bevy_reflect::{Reflect, Type, Typed as _};
pub use common_model as common;
use registry::registry::entry::Entry;
use registry::registry::singleton::Singleton;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod registries;
pub mod settings;

pub mod types;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(rename_all = "snake_case")]
pub enum ModModel {
    // registries
    Spaceship(Entry<SpaceshipModel>),
    Device(Entry<DeviceModel>),
    UnitDefinition(Entry<UnitDefinitionModel>),
    UnitVariable(Entry<UnitVariableModel>),
    // settings
    GameSetting(Singleton<GameSettings>),
}

impl ModModel {
    #[must_use]
    pub fn required_singletons() -> Vec<&'static Type> {
        vec![GameSettings::type_info().ty()]
    }
}
