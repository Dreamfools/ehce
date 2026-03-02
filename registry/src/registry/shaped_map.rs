use bevy_reflect::Type;
use std::any::{Any, TypeId};
use std::fmt::{Debug, Formatter};
use std::hash::{BuildHasher as _, BuildHasherDefault, Hasher};

#[allow(clippy::disallowed_types)]
pub type ReflectTypeMap<T> =
    std::collections::HashMap<TypeId, ReflectTypeStorage<T>, BuildHasherDefault<TypeIdHasher>>;

pub fn shaped_default<'a, T: Default>(map: &'a mut ReflectTypeMap<T>, ty: &Type) -> &'a mut T {
    map.entry(ty.id())
        .or_insert_with(|| ReflectTypeStorage {
            data: T::default(),
            ty: *ty,
        })
        .data_mut()
}

pub struct ReflectTypeStorage<T> {
    data: T,
    ty: Type,
}

impl<T> ReflectTypeStorage<T> {
    pub fn new(data: T, shape: &Type) -> Self {
        Self { data, ty: *shape }
    }

    pub fn ty(&self) -> &Type {
        &self.ty
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

impl<T: Debug> Debug for ReflectTypeStorage<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShapedStorage")
            .field("ty", &self.ty.path())
            .field("data", &self.data)
            .finish()
    }
}

/// A hasher optimized for hashing a single TypeId
///
/// Yoinked from `hecs` crate under MIT license.
///
/// TypeId is already thoroughly hashed, so there's no reason to hash it again.
/// Just leave the bits unchanged.
#[derive(Default)]
pub struct TypeIdHasher {
    hash: u64,
}

impl Hasher for TypeIdHasher {
    fn finish(&self) -> u64 {
        self.hash
    }

    fn write(&mut self, bytes: &[u8]) {
        debug_assert_eq!(self.hash, 0);

        // This will only be called if TypeId is neither u64 nor u128, which is not anticipated.
        // In that case we'll just fall back to using a different hash implementation.
        let mut hasher = foldhash::fast::FixedState::with_seed(0xb334867b740a29a5).build_hasher();
        hasher.write(bytes);
        self.hash = hasher.finish();
    }

    fn write_u64(&mut self, n: u64) {
        // Only a single value can be hashed, so the old hash should be zero.
        debug_assert_eq!(self.hash, 0);
        self.hash = n;
    }

    // Tolerate TypeId being either u64 or u128.
    fn write_u128(&mut self, n: u128) {
        debug_assert_eq!(self.hash, 0);
        self.hash = n as u64;
    }
}
