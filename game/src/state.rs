use bevy::app::{App, Plugin};
use bevy::log::error;
use bevy::prelude::{Commands, OnEnter, OnExit, Res, Resource, State, States};
use bevy::state::state::FreelyMutableState;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum GameState {
    /// Critical unrecoverable error state
    Error,
    /// Application initialization state
    #[default]
    Init,
    /// Gameplay state
    Gameplay,
}

impl States for GameState {}
impl FreelyMutableState for GameState {}

/// Simple plugin to assert the presence of a resource when entering a state
/// and remove it when exiting the state.
pub struct SimpleStateObjectPlugin<S: States + Clone, T: Resource>(S, PhantomData<T>);

impl<S: States + Clone, T: Resource> SimpleStateObjectPlugin<S, T> {
    pub fn new(state: S) -> Self {
        Self(state, Default::default())
    }
}

impl<S: States + Clone, T: Resource> Plugin for SimpleStateObjectPlugin<S, T> {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(self.0.clone()), assert_state_object::<S, T>)
            .add_systems(OnExit(self.0.clone()), cleanup_state_object::<T>);
    }
}

pub fn assert_state_object<S: States + Debug, T: Resource>(
    res: Option<Res<T>>,
    state: Res<State<S>>,
) {
    if res.is_none() {
        error!(
            ?state,
            "State object is missing after transitioning to a state"
        )
    }
}

pub fn cleanup_state_object<T: Resource>(mut commands: Commands) {
    commands.remove_resource::<T>();
}
