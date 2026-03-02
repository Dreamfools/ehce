use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::traverse::TraverseKind;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[reflect(@TraverseKind::Singleton)]
#[serde(transparent)]
pub struct Singleton<T: Sized>(T);

impl<T: Sized> Singleton<T> {
    pub fn new(item: T) -> Self {
        Self(item)
    }

    pub fn item(&self) -> &T {
        &self.0
    }
}