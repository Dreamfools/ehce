use bevy::prelude::States;
use bevy::state::state::FreelyMutableState;
use std::fmt::Debug;

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