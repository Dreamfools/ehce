use crate::path::FieldPath;
use bevy_reflect::{Reflect, Reflectable, Type, TypeInfo};
use id::RawId;

pub mod entry;
pub mod id;
pub mod reflect_registry;
pub mod shaped_map;
pub mod singleton;

pub trait TraverseRegistry {
    fn id_ref_seen(
        &mut self,
        path: &FieldPath,
        ty_shape: &Type,
        id: RawId,
    ) -> rootcause::Result<()>;

    fn consume_entry(
        &mut self,
        path: &FieldPath,
        ty_shape: &Type,
        id: RawId,
        entry: Box<dyn Reflect>,
    ) -> rootcause::Result<()>;

    fn consume_singleton(
        &mut self,
        path: &FieldPath,
        ty_shape: &Type,
        singleton: Box<dyn Reflect>,
    ) -> rootcause::Result<()>;
}

/// A convenience function to register an ID reference in the registry.
pub fn id_ref_seen<T: Reflectable>(
    registry: &mut impl TraverseRegistry,
    path: &FieldPath,
    id: RawId,
) -> rootcause::Result<()> {
    let ty = T::type_info().ty();
    registry.id_ref_seen(path, ty, id)
}

/// A convenience function to register an entry in the registry.
pub fn consume_entry<T: Reflectable>(
    registry: &mut impl TraverseRegistry,
    path: &FieldPath,
    id: RawId,
    entry: T,
) -> rootcause::Result<()> {
    let shape = T::type_info().ty();
    registry.consume_entry(path, shape, id, Box::new(entry))
}
