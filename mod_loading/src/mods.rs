use bevy::app::{App, First, Plugin};
use bevy::prelude::{AppExtStates as _, Message, Reflect, Resource, States, SystemSet};
use bevy::state::state::FreelyMutableState;
use registry::registry::reflect_registry::ReflectRegistry;
use rootcause::Report;
use crate::loading::ModLoadingPlugin;

#[derive(Debug)]
pub struct ModPlugin;

impl Plugin for ModPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(First, HotReloadingSystems);
        app.insert_state(ModState::default())
            .add_message::<WantLoadModMessage>()
            .add_message::<ModLoadErrorMessage>()
            .add_message::<ModLoadedMessage>()
            .add_plugins(ModLoadingPlugin);
    }
}
#[derive(Debug, Reflect, Resource)]
pub struct ModData {
    pub registry: ReflectRegistry,
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
#[derive(Debug, Reflect, Message)]
pub struct WantLoadModMessage;

/// Message that is triggered when mod loading fails for any reason
///
/// This event should not be raised outside of mod loading code
///
/// Errors are logged via error!, so use custom tracing frontend to report
/// errors to the user
#[derive(Debug, Message)]
#[cfg_attr(bevy_lint, allow(bevy::missing_reflect))]
pub struct ModLoadErrorMessage(pub Report);

/// Message that is triggered when mod is loaded successfully
///
/// At any point in an app lifecycle, there should only be one system listening
/// for this event, and it should drain this event as soon as possible
///
/// Payload is a full mod data
#[derive(Debug, Reflect, Message)]
pub struct ModLoadedMessage(pub ModData);

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct HotReloadingSystems;
