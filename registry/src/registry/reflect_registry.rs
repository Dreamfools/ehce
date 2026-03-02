use crate::path::FieldPath;
use crate::registry::TraverseRegistry;
use crate::registry::id::{IdRef, RawId};
use crate::registry::shaped_map::{ReflectTypeMap, ReflectTypeStorage, shaped_default};
use bevy_reflect::{Reflect, Reflectable, Type};
use itertools::Itertools as _;
use std::any::{Any, TypeId};
use std::collections::BTreeSet;
use std::collections::hash_map::Entry;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Index;
use ustr::{UstrMap, UstrSet};

#[derive(Debug)]
pub struct ReflectRegistry {
    ids: ReflectTypeMap<UstrMap<Vec<FieldPath>>>,
    entries: ReflectTypeMap<UstrMap<(FieldPath, Box<dyn Reflect>)>>,
    singletons: ReflectTypeMap<(FieldPath, Box<dyn Reflect>)>,
}

impl ReflectRegistry {
    #[must_use]
    pub fn singleton<T: Sized + Reflectable>(&self) -> &T {
        self.try_singleton()
            .expect("Singleton of type T should be present in the registry")
    }

    #[must_use]
    pub fn try_singleton<T: Sized + Reflectable>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let storage = self.singletons.get(&type_id)?;

        debug_assert_eq!(T::type_info().ty(), storage.ty());
        let ptr = &storage.data().1;

        Some(ptr.downcast_ref().expect("Singleton type matches T"))
    }

    /// Returns a reference to the entry with the given `id` and type `T`
    #[must_use]
    pub fn get<T: Sized + Reflectable>(&self, id: IdRef<T>) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let storage = self.entries.get(&type_id)?;

        debug_assert_eq!(T::type_info().ty(), storage.ty());
        let ptr = storage.data().get(&id.raw().0)?;

        Some(ptr.1.downcast_ref().expect("Entry type matches T"))
    }

    /// Returns an iterator over all entries of type `T` in the registry
    pub fn iter<T: Sized + Reflectable>(
        &self,
    ) -> impl Iterator<Item = (IdRef<T>, (&FieldPath, &T))> {
        let type_id = TypeId::of::<T>();
        self.entries.get(&type_id).into_iter().flat_map(|storage| {
            debug_assert_eq!(T::type_info().ty(), storage.ty());

            storage.data().iter().map(move |(id, (path, ptr))| {
                let reference = ptr.downcast_ref::<T>().expect("Entry type matches T");
                (IdRef::<T>::new(RawId::new(*id)), (path, reference))
            })
        })
    }

    #[must_use]
    pub fn has<T: Sized + Reflectable>(&self, id: IdRef<T>) -> bool {
        let type_id = TypeId::of::<T>();
        self.has_raw(type_id, id.raw())
    }

    #[must_use]
    pub fn ids_equal(&self, other: &Self) -> bool {
        for (ty, store) in &self.ids {
            let Some(other_store) = other.ids.get(ty) else {
                if !store.data().is_empty() {
                    return false; // other registry does not have this type
                }
                continue;
            };

            if store.data() != other_store.data() {
                return false;
            }
        }

        for (ty, store) in &other.entries {
            let Some(other_store) = self.entries.get(ty) else {
                if !store.data().is_empty() {
                    return false; // self registry does not have this type
                }
                continue;
            };

            let other_data = other_store.data();
            for (k, v) in store.data().iter() {
                if other_data.get(k).is_none_or(|ov| ov.0 != v.0) {
                    return false; // entries do not match
                }
            }
        }

        if self.singletons.keys().collect::<BTreeSet<_>>()
            != other.singletons.keys().collect::<BTreeSet<_>>()
        {
            return false; // singletons presence does not match
        }

        true
    }

    pub fn item_differences<T: Sized + Reflectable + PartialEq>(
        &self,
        other: &Self,
    ) -> impl Iterator<Item = IdRef<T>> {
        let mut differences = UstrSet::default();
        for (id, (_, item)) in self.iter::<T>() {
            let other_item = other.get(id);
            if Some(item) != other_item {
                differences.insert(id.raw().0);
            }
        }
        for (id, (_, item)) in other.iter::<T>() {
            if differences.contains(&id.raw().0) {
                continue; // already found in the first loop
            }
            let self_item = self.get(id);
            if Some(item) != self_item {
                differences.insert(id.raw().0);
            }
        }

        differences
            .into_iter()
            .map(|id| IdRef::<T>::new(RawId::new(id.as_str())))
    }

    fn has_raw(&self, type_id: TypeId, id: &RawId) -> bool {
        if let Some(storage) = self.entries.get(&type_id) {
            storage.data().contains_key(&id.0)
        } else {
            false
        }
    }
}

impl<T: Sized + Reflectable> Index<IdRef<T>> for ReflectRegistry {
    type Output = T;

    fn index(&self, id: IdRef<T>) -> &Self::Output {
        self.get(id).expect("ID should be present in the registry")
    }
}

impl<T: Sized + Reflectable> Index<&IdRef<T>> for ReflectRegistry {
    type Output = T;

    fn index(&self, id: &IdRef<T>) -> &Self::Output {
        self.get(*id).expect("ID should be present in the registry")
    }
}

#[derive(Debug)]
pub struct BuildRegistryError {
    pub missing_ids: ReflectTypeMap<UstrMap<Vec<FieldPath>>>,
    pub duplicate_entries: ReflectTypeMap<UstrMap<Vec<FieldPath>>>,
    pub duplicate_singletons: ReflectTypeMap<Vec<FieldPath>>,
    pub missing_singletons: Vec<Type>,
}

impl Display for BuildRegistryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fn name_of(ty: &Type) -> &str {
            ty.short_path()
        }
        if !self.missing_ids.is_empty() {
            write!(f, "Missing IDs:")?;
            for ids in self.missing_ids.values() {
                write!(f, "\n  {}: ", name_of(ids.ty()))?;
                for (id, paths) in ids.data() {
                    write!(f, "\n    `{}` ({})", id, paths.len())?;
                    if !paths.is_empty() {
                        write!(
                            f,
                            " at {}",
                            paths.iter().map(FieldPath::format_path).join(", ")
                        )?;
                    }
                }
            }
        }

        if !self.duplicate_entries.is_empty() {
            write!(f, "Duplicate entries:")?;
            for entries in self.duplicate_entries.values() {
                write!(f, "\n  {}: ", name_of(entries.ty()))?;
                for (id, paths) in entries.data() {
                    if !paths.is_empty() {
                        write!(f, "\n    `{}` ({})", id, paths.len())?;
                        write!(
                            f,
                            " at {}",
                            paths.iter().map(FieldPath::format_path).join(", ")
                        )?;
                    }
                }
            }
        }

        if !self.duplicate_singletons.is_empty() {
            write!(f, "Duplicate singletons:")?;
            for paths in self.duplicate_singletons.values() {
                write!(f, "\n  {}: ", name_of(paths.ty()))?;
                for path in paths.data() {
                    write!(f, "{}", path.format_path())?;
                }
            }
        }

        if !self.missing_singletons.is_empty() {
            write!(f, "Missing singletons:")?;
            for ty in &self.missing_singletons {
                write!(f, "\n  {}", name_of(ty))?;
            }
        }

        Ok(())
    }
}

impl Error for BuildRegistryError {}

#[derive(Debug)]
pub struct BuildReflectRegistry {
    registry: ReflectRegistry,
    error: BuildRegistryError,
}

impl Default for BuildReflectRegistry {
    fn default() -> Self {
        Self {
            registry: ReflectRegistry {
                ids: Default::default(),
                entries: Default::default(),
                singletons: Default::default(),
            },
            error: BuildRegistryError {
                missing_ids: Default::default(),
                duplicate_entries: Default::default(),
                duplicate_singletons: Default::default(),
                missing_singletons: Default::default(),
            },
        }
    }
}

impl BuildReflectRegistry {
    /// Returns the built registry if there are no ID errors, otherwise returns
    /// an error containing information about ID errors
    pub fn build(mut self) -> Result<ReflectRegistry, BuildRegistryError> {
        // dbg!(&self);
        self.error.missing_ids.retain(|_, v| !v.data().is_empty());
        self.error
            .duplicate_entries
            .retain(|_, v| !v.data().is_empty());
        self.error
            .missing_singletons
            .retain(|s| !self.registry.singletons.contains_key(&s.id()));
        if self.error.missing_ids.is_empty()
            && self.error.duplicate_entries.is_empty()
            && self.error.missing_singletons.is_empty()
            && self.error.duplicate_singletons.is_empty()
        {
            Ok(self.registry)
        } else {
            Err(self.error)
        }
    }

    /// Indicates that the registry expects singletons of the given type to
    /// be present
    pub fn expect_singletons(&mut self, types: Vec<&Type>) {
        self.error.missing_singletons.extend(types.iter().copied());
    }
}

impl TraverseRegistry for BuildReflectRegistry {
    fn id_ref_seen(
        &mut self,
        path: &FieldPath,
        ty_shape: &Type,
        id: RawId,
    ) -> rootcause::Result<()> {
        let ty = ty_shape.id();

        shaped_default(&mut self.registry.ids, ty_shape)
            .entry(id.0)
            .or_default()
            .push(path.clone());

        if self.registry.has_raw(ty, &id) {
            // id is already present, do nothing
            return Ok(());
        }
        shaped_default(&mut self.error.missing_ids, ty_shape)
            .entry(id.0)
            .or_default()
            .push(path.clone());
        Ok(())
    }

    fn consume_entry(
        &mut self,
        path: &FieldPath,
        ty_shape: &Type,
        id: RawId,
        entry: Box<dyn Reflect>,
    ) -> rootcause::Result<()> {
        debug_assert_eq!(
            Some(ty_shape),
            entry.get_represented_type_info().map(|i| i.ty())
        );

        let ty = ty_shape.id();

        let storage = shaped_default(&mut self.registry.entries, ty_shape);

        match storage.entry(id.0) {
            Entry::Occupied(e) => {
                let duplicates = shaped_default(&mut self.error.duplicate_entries, ty_shape)
                    .entry(*e.key())
                    .or_default();
                if duplicates.is_empty() {
                    duplicates.push(e.get().0.clone());
                }
                duplicates.push(path.clone());
            }
            Entry::Vacant(e) => {
                e.insert((path.clone(), entry));
            }
        }

        if let Some(ids) = self.error.missing_ids.get_mut(&ty) {
            ids.data_mut().remove(&id.0);
        }

        Ok(())
    }

    fn consume_singleton(
        &mut self,
        path: &FieldPath,
        ty_shape: &Type,
        singleton: Box<dyn Reflect>,
    ) -> rootcause::Result<()> {
        debug_assert_eq!(
            Some(ty_shape),
            singleton.get_represented_type_info().map(|i| i.ty())
        );

        let ty = ty_shape.id();

        match self.registry.singletons.entry(ty) {
            Entry::Vacant(e) => {
                e.insert(ReflectTypeStorage::new((path.clone(), singleton), ty_shape));
                Ok(())
            }
            Entry::Occupied(e) => {
                let duplicates = shaped_default(&mut self.error.duplicate_singletons, ty_shape);
                if duplicates.is_empty() {
                    duplicates.push(e.get().data().0.clone());
                }
                duplicates.push(path.clone());
                Ok(())
            }
        }
    }
}
