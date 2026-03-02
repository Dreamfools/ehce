use crate::path::{FieldPath, Segment};
use crate::registry::id::RawId;
use crate::registry::TraverseRegistry;
use bevy_reflect::{PartialReflect, Reflect, ReflectKind, TypeInfo, VariantField, VariantType};
use rootcause::bail;

pub fn traverse<T: Reflect>(
    item: &T,
    path: &FieldPath,
    registry: &mut impl TraverseRegistry,
) -> rootcause::Result<()> {
    let mut path = path.clone();
    traverse_owning(item.as_reflect(), &mut path, registry)
}

#[derive(Debug, Eq, PartialEq, Reflect)]
pub(crate) enum TraverseKind {
    IdRef,
    Entry,
    Singleton,
}

fn traverse_owning(
    item: &dyn Reflect,
    path: &mut FieldPath,
    registry: &mut impl TraverseRegistry,
) -> rootcause::Result<()> {
    let info = item.reflect_type_info();
    match info {
        TypeInfo::Struct(data) => {
            let attrs = data.custom_attributes();
            let kind = attrs.get::<TraverseKind>();
            let Some(TraverseKind::Entry) = kind else {
                bail!("Traversed struct is not an Entry or Singleton");
            };
            let item = item.reflect_ref().as_struct().expect("Entry is a struct");
            let id_field = item.field("id").expect("Entry has `id` field as RawId");
            let id = id_field
                .try_downcast_ref::<RawId>()
                .expect("Entry `id` field is a RawId");
            let data_field = item.field("data").expect("Entry has `data` field as value");

            // TODO: there should be owned way to get field's value
            let data_field = data_field
                .reflect_clone()
                .expect("All values should be clonable")
                .try_into_reflect()
                .expect("Entry's data field is a concrete type");

            traverse_inner(&*data_field, path, registry)?;
            registry.consume_entry(path, data_field.reflect_type_info().ty(), *id, data_field)?;
            Ok(())
        }
        TypeInfo::TupleStruct(data) => {
            let attrs = data.custom_attributes();
            let kind = attrs.get::<TraverseKind>();
            let Some(TraverseKind::Singleton) = kind else {
                bail!("Traversed struct is not an Entry or Singleton");
            };

            let item = item
                .reflect_ref()
                .as_tuple_struct()
                .expect("Singleton is a tuple struct");
            let data_field = item
                .field(0)
                .expect("Singleton has the only field as value");
            // TODO: there should be owned way to get field's value?
            let data_field = data_field
                .reflect_clone()
                .expect("All values should be clonable")
                .try_into_reflect()
                .expect("Singleton's data field is a concrete type");
            traverse_inner(&*data_field, path, registry)?;
            registry.consume_singleton(path, data_field.reflect_type_info().ty(), data_field)?;
            Ok(())
        }
        TypeInfo::Enum(_) => {
            let item = item.reflect_ref().as_enum().expect("Entry is enum");
            if item.field_len() != 1 {
                bail!("Traversed enum must have exactly one field in each variant");
            }
            if item.variant_type() != VariantType::Tuple {
                bail!("Traversed enum variants must be tuple variants");
            }
            let variant_name = item.variant_name();
            let data_field = item
                .field_at(0)
                .expect("Enum variant has one field")
                .try_as_reflect()
                .expect("Enum variant field is a concrete type");

            path.with_segment(Segment::EnumVariant(variant_name.to_string()), |path| {
                traverse_owning(data_field, path, registry)
            })
        }
        _ => {
            bail!(
                "Traversed values must either be Entry, Singleton, or enum containing either of those"
            );
        }
    }
}
fn traverse_inner(
    base_item: &dyn PartialReflect,
    path: &mut FieldPath,
    registry: &mut impl TraverseRegistry,
) -> rootcause::Result<()> {
    match base_item.reflect_kind() {
        ReflectKind::Struct => {
            let item = base_item
                .reflect_ref()
                .as_struct()
                .expect("reflect kind is struct");
            let attrs = base_item
                .get_represented_type_info()
                .expect("Struct has represented type info")
                .as_struct()
                .expect("represented type info is struct")
                .custom_attributes();
            if let Some(kind) = attrs.get::<TraverseKind>() {
                let TraverseKind::IdRef = kind else {
                    bail!("Entry or Singleton found during inner traversal");
                };
                let id_field = item.field("id").expect("IdRef has `id` field as RawId");
                let id = id_field
                    .try_downcast_ref::<RawId>()
                    .expect("IdRef `id` field is a RawId");
                let info = base_item
                    .get_represented_type_info()
                    .expect("IdRef has represented type info");
                let generic_ty = info
                    .generics().first()
                    .expect("IdRef has one generic type argument")
                    .ty();

                registry.id_ref_seen(path, generic_ty, *id)?;
            } else {
                for (i, value) in item.iter_fields().enumerate() {
                    let name = item.name_at(i).expect("field has a name");
                    path.with_segment(Segment::Field(name.to_string()), |path| {
                        traverse_inner(value, path, registry)
                    })?;
                }
            }
        }
        ReflectKind::TupleStruct => {
            let item = base_item
                .reflect_ref()
                .as_tuple_struct()
                .expect("reflect kind is tuple struct");

            if cfg!(debug_assertions) {
                let attrs = base_item
                    .get_represented_type_info()
                    .expect("Struct has represented type info")
                    .as_tuple_struct()
                    .expect("represented type info is tuple struct")
                    .custom_attributes();

                if attrs.contains::<TraverseKind>() {
                    bail!("Singleton found during inner traversal");
                }
            }

            for (i, value) in item.iter_fields().enumerate() {
                path.with_segment(Segment::TupleIndex(i), |path| {
                    traverse_inner(value, path, registry)
                })?;
            }
        }
        ReflectKind::Tuple => {
            let item = base_item
                .reflect_ref()
                .as_tuple()
                .expect("reflect kind is tuple");
            for (i, value) in item.iter_fields().enumerate() {
                path.with_segment(Segment::TupleIndex(i), |path| {
                    traverse_inner(value, path, registry)
                })?;
            }
        }
        ReflectKind::List => {
            let item = base_item
                .reflect_ref()
                .as_list()
                .expect("reflect kind is list");
            for (i, value) in item.iter().enumerate() {
                path.with_segment(Segment::ListIndex(i), |path| {
                    traverse_inner(value, path, registry)
                })?;
            }
        }
        ReflectKind::Array => {
            let item = base_item
                .reflect_ref()
                .as_array()
                .expect("reflect kind is array");
            for (i, value) in item.iter().enumerate() {
                path.with_segment(Segment::ListIndex(i), |path| {
                    traverse_inner(value, path, registry)
                })?;
            }
        }
        ReflectKind::Map => {
            let item = base_item
                .reflect_ref()
                .as_map()
                .expect("reflect kind is map");
            for (k, v) in item.iter() {
                path.with_segment(Segment::MapKey, |path| traverse_inner(k, path, registry))?;
                let key_name = name_of_reflect(k);
                path.with_segment(Segment::MapEntry(key_name), |path| {
                    traverse_inner(v, path, registry)
                })?;
            }
        }
        ReflectKind::Set => {
            let item = base_item
                .reflect_ref()
                .as_set()
                .expect("reflect kind is set");
            for v in item.iter() {
                path.with_segment(Segment::SetEntry, |path| traverse_inner(v, path, registry))?;
            }
        }
        ReflectKind::Enum => {
            let item = base_item
                .reflect_ref()
                .as_enum()
                .expect("reflect kind is enum");
            let variant_name = item.variant_name();
            path.with_segment(
                Segment::EnumVariant(variant_name.to_string()),
                |path| -> rootcause::Result<()> {
                    for (i, value) in item.iter_fields().enumerate() {
                        match value {
                            VariantField::Struct(name, value) => {
                                path.with_segment(Segment::Field(name.to_string()), |path| {
                                    traverse_inner(value, path, registry)
                                })?;
                            }
                            VariantField::Tuple(value) => {
                                path.with_segment(Segment::TupleIndex(i), |path| {
                                    traverse_inner(value, path, registry)
                                })?;
                            }
                        }
                    }
                    Ok(())
                },
            )?;
        }
        ReflectKind::Opaque => {
            // nothing to do for opaques
        }
    }
    Ok(())
}
//
fn name_of_reflect(k: &dyn PartialReflect) -> String {
    if let Some(name) = k.try_downcast_ref::<String>() {
        name.clone()
    } else {
        format!("{:?}", k)
    }
}
