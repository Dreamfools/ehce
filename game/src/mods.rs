use std::path::PathBuf;
use bevy::app::{App, First, Plugin};
use bevy::asset::{Handle, LoadedFolder};
use bevy::prelude::{AppExtStates, Message, Resource, States, SystemSet};
use bevy::state::state::FreelyMutableState;
use registry::registry::id::{IdRef, RawId};
use registry::registry::reflect_registry::ReflectRegistry;
use crate::loading::ModLoadingPlugin;

#[derive(Debug)]
pub struct ModPlugin;

impl Plugin for ModPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(First, HotReloading);
        app.insert_state(ModState::default())
            .add_message::<WantLoadModMessage>()
            .add_message::<ModLoadErrorMessage>()
            .add_message::<ModLoadedMessage>()
            .add_plugins(ModLoadingPlugin);
    }
}
#[derive(Debug, Resource)]
pub struct ModData {
    pub name: String,
    pub registry: ReflectRegistry,
    pub mod_path: PathBuf,
    pub folder_handle: Handle<LoadedFolder>,
    // pub assets: FxBiHashMap<Utf8PathBuf, RegistryId>,
}
#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub enum ModState {
    /// Default state, signifying that no mod is loaded
    #[default]
    None,
    /// State signifying that a mod is loading
    Loading,
    /// State signifying that a mod loading is finished and awaiting handling
    /// from the current state
    Pending,
    /// State signifying a loaded mod, and listening for hot reload events
    Ready,
}

impl States for ModState {}

impl FreelyMutableState for ModState {}

/// Message that triggers loading of a new mod
///
/// Should generally be only raised by app code, but not listened to
#[derive(Debug, Message)]
pub struct WantLoadModMessage(pub String);

/// Message that is triggered when mod loading fails for any reason
///
/// This event should not be raised outside of mod loading code
///
/// Errors are logged via error!, so use custom tracing frontend to report
/// errors to the user
#[derive(Debug, Message)]
pub struct ModLoadErrorMessage(pub String);

/// Message that is triggered when mod is loaded successfully
///
/// At any point in an app lifecycle, there should only be one system listening
/// for this event, and it should drain this event as soon as possible
///
/// Payload is a full mod data
#[derive(Debug, Message)]
pub struct ModLoadedMessage(pub ModData);

/// Message that is triggered when hot reload happens
#[derive(Debug, Message)]
pub struct ModHotReloadMessage;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct HotReloading;