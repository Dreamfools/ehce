use crate::registry::id::RawId;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::de::{DeserializeSeed, Error, MapAccess, Visitor};
use serde::{de, forward_to_deserialize_any, Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt::Formatter;
use std::marker::PhantomData;
use bevy_reflect::Reflect;
use crate::traverse::TraverseKind;

#[derive(Debug, Clone, Reflect)]
#[reflect(@TraverseKind::Entry)]
pub struct Entry<T: Sized> {
    id: RawId,
    data: T,
}

impl<T: Sized> Entry<T> {
    pub fn new(id: RawId, data: T) -> Self {
        Self { id, data }
    }

    pub fn id(&self) -> &RawId {
        &self.id
    }

    pub fn item(&self) -> &T {
        &self.data
    }
}

const _: () = {
    impl<T: Sized + JsonSchema> JsonSchema for Entry<T> {
        fn schema_name() -> Cow<'static, str> {
            format!("Entry<{}>", T::schema_name()).into()
        }

        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            EntrySchema::<T>::json_schema(generator)
        }
    }

    #[allow(dead_code)]
    #[derive(JsonSchema)]
    struct EntrySchema<T> {
        pub id: RawId,
        #[serde(flatten)]
        pub data: T,
    }
    impl<T: Sized + Serialize> Serialize for Entry<T> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            #[derive(Serialize)]
            struct EntryRepr<'a, T> {
                id: &'a RawId,
                #[serde(flatten)]
                data: &'a T,
            }

            EntryRepr {
                id: &self.id,
                data: &self.data,
            }
            .serialize(serializer)
        }
    }

    impl<'de, T: Sized + Deserialize<'de> + 'de> Deserialize<'de> for Entry<T> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct StrDeserializer<'a, E> {
                value: &'a str,
                marker: PhantomData<E>,
            }

            impl<'de, 'a, E> Deserializer<'de> for StrDeserializer<'a, E>
            where
                E: Error,
            {
                type Error = E;

                fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    visitor.visit_str(self.value)
                }

                forward_to_deserialize_any! {
                    bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
                    bytes byte_buf option unit unit_struct newtype_struct seq tuple
                    tuple_struct map struct enum identifier ignored_any
                }
            }

            struct IdVisitMapAccess<'de, A: MapAccess<'de>> {
                map: A,
                id: Option<RawId>,
                _lifetime: PhantomData<&'de ()>,
            }

            impl<'de, A: MapAccess<'de>> MapAccess<'de> for &mut IdVisitMapAccess<'de, A> {
                type Error = A::Error;

                fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
                where
                    K: DeserializeSeed<'de>,
                {
                    let key = self.map.next_key::<String>()?;
                    let Some(key) = key else {
                        return Ok(None);
                    };

                    if key == "id" {
                        let id = self.map.next_value::<RawId>()?;
                        self.id = Some(id);
                        self.next_key_seed(seed)
                    } else {
                        let de = StrDeserializer {
                            value: &key,
                            marker: Default::default(),
                        };
                        Ok(Some(seed.deserialize(de)?))
                    }
                }

                fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
                where
                    V: DeserializeSeed<'de>,
                {
                    self.map.next_value_seed(seed)
                }
            }

            struct EntryVisitor<'de, T: Sized + Deserialize<'de>>(PhantomData<&'de fn() -> T>);
            impl<'de, T: Sized + Deserialize<'de>> Visitor<'de> for EntryVisitor<'de, T> {
                type Value = Entry<T>;

                fn expecting(&self, f: &mut Formatter) -> std::fmt::Result {
                    write!(f, "map")
                }

                fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    let mut access = IdVisitMapAccess {
                        map,
                        id: None,
                        _lifetime: Default::default(),
                    };

                    let de = de::value::MapAccessDeserializer::new(&mut access);

                    let data = T::deserialize(de)?;
                    let id = access.id.ok_or_else(|| A::Error::missing_field("id"))?;

                    Ok(Entry::new(id, data))
                }
            }

            deserializer.deserialize_map(EntryVisitor(PhantomData))
        }
    }
};
