use bevy_reflect::{Reflect, Reflectable, TypePath};
use registry::registry::id::{IdRef, RawId};
use registry::registry::reflect_registry::ReflectRegistry;
use schemars::JsonSchema;
use serde::de::{EnumAccess, Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Formatter;

#[derive(Debug, Clone, Reflect)]
#[repr(C)]
pub enum RefOrInline<T> {
    Ref(IdRef<T>),
    Inline(T),
}

impl<T: Sized + Reflectable> RefOrInline<T> {
    pub fn get<'a>(&'a self, registry: &'a ReflectRegistry) -> &'a T {
        match self {
            RefOrInline::Ref(id) => &registry[id],
            RefOrInline::Inline(value) => value,
        }
    }
}

const _: () = {
    #[derive(Debug, Clone, Serialize, JsonSchema)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum RefOrInlineSchema<T: TypePath> {
        Ref(IdRef<T>),
        Inline(T),
    }

    impl<T: JsonSchema + TypePath> JsonSchema for RefOrInline<T> {
        fn schema_name() -> std::borrow::Cow<'static, str> {
            format!("{}RefOrInline", T::schema_name()).into()
        }

        fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
            RefOrInlineSchema::<T>::json_schema(generator)
        }
    }

    impl<T: Serialize> Serialize for RefOrInline<T> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                RefOrInline::Ref(id) => id.serialize(serializer),
                RefOrInline::Inline(value) => value.serialize(serializer),
            }
        }
    }

    impl<'de, T: Deserialize<'de>> Deserialize<'de> for RefOrInline<T> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct RefOrInlineVisitor<T> {
                marker: std::marker::PhantomData<fn() -> T>,
            }

            impl<'de, T: Deserialize<'de>> Visitor<'de> for RefOrInlineVisitor<T> {
                type Value = RefOrInline<T>;

                fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                    write!(formatter, "an id string or an inline value")
                }

                fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    let de = serde::de::value::BoolDeserializer::new(v);
                    Ok(RefOrInline::Inline(T::deserialize(de)?))
                }

                fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    let de = serde::de::value::I64Deserializer::new(v);
                    Ok(RefOrInline::Inline(T::deserialize(de)?))
                }

                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    let de = serde::de::value::U64Deserializer::new(v);
                    Ok(RefOrInline::Inline(T::deserialize(de)?))
                }

                fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    let de = serde::de::value::F64Deserializer::new(v);
                    Ok(RefOrInline::Inline(T::deserialize(de)?))
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(RefOrInline::Ref(IdRef::new(RawId::new(v))))
                }

                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    let de = serde::de::value::BytesDeserializer::new(v);
                    Ok(RefOrInline::Inline(T::deserialize(de)?))
                }

                fn visit_none<E>(self) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    let de = serde::de::value::UnitDeserializer::new();
                    Ok(RefOrInline::Inline(T::deserialize(de)?))
                }

                fn visit_some<D>(self, de: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    Ok(RefOrInline::Inline(T::deserialize(de)?))
                }

                fn visit_unit<E>(self) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    let de = serde::de::value::UnitDeserializer::new();
                    Ok(RefOrInline::Inline(T::deserialize(de)?))
                }

                fn visit_newtype_struct<D>(self, de: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    Ok(RefOrInline::Inline(T::deserialize(de)?))
                }

                fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let de = serde::de::value::SeqAccessDeserializer::new(seq);
                    Ok(RefOrInline::Inline(T::deserialize(de)?))
                }

                fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    let de = serde::de::value::MapAccessDeserializer::new(map);
                    Ok(RefOrInline::Inline(T::deserialize(de)?))
                }

                fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
                where
                    A: EnumAccess<'de>,
                {
                    let de = serde::de::value::EnumAccessDeserializer::new(data);
                    Ok(RefOrInline::Inline(T::deserialize(de)?))
                }
            }

            deserializer.deserialize_any(RefOrInlineVisitor {
                marker: std::marker::PhantomData,
            })
        }
    }
};
