use bevy_reflect::Reflect;
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};
use glam::{
    I64Vec2, I64Vec3, I64Vec4, IVec2, IVec3, IVec4, U64Vec2, U64Vec3, U64Vec4, UVec2, UVec3, UVec4,
};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt::Formatter;
use std::ops::Deref;

macro_rules! count_tts {
    () => { 0 };
    ($odd:tt $($a:tt $b:tt)*) => { (count_tts!($($a)*) << 1) | 1 };
    ($($a:tt $even:tt)*) => { count_tts!($($a)*) << 1 };
}

macro_rules! wrap_model {
    ($data_ty:ty, $ty:ident, $model:ident {$($field:ident),*}) => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect)]
        #[reflect(opaque, Clone, Debug, PartialEq, Hash, Serialize, Deserialize)]
        pub struct $model($ty);

        const _: () = {
            #[derive(Debug, Copy, Clone, Serialize, Deserialize, JsonSchema)]
            #[repr(C)]
            #[serde(untagged)]
            pub enum $ty{
                Struct{
                    $(
                        $field: $data_ty,
                    )*
                },
                Array([$data_ty; count_tts!($($field)*)]),
            }

            impl Serialize for $model {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                        where
                            S: Serializer,
                        {
                            self.0.serialize(serializer)
                        }
            }

            impl <'de> Deserialize<'de> for $model {
                fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
                {
                    struct VisitorImpl;
                    impl<'de> Visitor<'de> for VisitorImpl {
                        type Value = $model;

                        fn expecting(&self, f: &mut Formatter) -> std::fmt::Result {
                            write!(f, "array or struct")
                        }

                        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
                        where
                            A: SeqAccess<'de>,
                        {
                            let de = serde::de::value::SeqAccessDeserializer::new(seq);
                            let [$($field),*] = <[$data_ty; count_tts!($($field)*)]>::deserialize(de)?;
                            Ok($model::new($($field),*))
                        }

                        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
                        where
                            A: MapAccess<'de>,
                        {
                            let de = serde::de::value::MapAccessDeserializer::new(map);
                            Ok($model(glam::$ty::deserialize(de)?))
                        }
                    }

                    deserializer.deserialize_any(VisitorImpl)
                }
            }

            impl JsonSchema for $model {
                fn schema_name() -> Cow<'static, str> {
                    $ty::schema_name()
                }

                fn json_schema(generator: &mut SchemaGenerator) -> Schema {
                    $ty::json_schema(generator)
                }
            }
        };

        impl Deref for $model {
            type Target = $ty;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<$model> for $ty {
            fn from(value: $model) -> Self {
                value.0
            }
        }

        impl Default for $model {
            fn default() -> Self {
                Self(Default::default())
            }
        }

        impl $model {
            #[must_use] pub fn new($($field: $data_ty),*) -> Self {
                Self($ty {
                    $($field),*
                })
            }

            #[must_use] pub fn glam(&self) -> $ty {
                Into::<$ty>::into(*self)
            }
        }
    };
}

pub enum Vec2Model {
    Struct { x: f32, y: f32 },
    Array([f32; 2]),
}

wrap_model!(i32, IVec2, IVec2Model { x, y });
wrap_model!(i32, IVec3, IVec3Model { x, y, z });
wrap_model!(i32, IVec4, IVec4Model { x, y, z, w });
wrap_model!(u32, UVec2, UVec2Model { x, y });
wrap_model!(u32, UVec3, UVec3Model { x, y, z });
wrap_model!(u32, UVec4, UVec4Model { x, y, z, w });
wrap_model!(i64, I64Vec2, I64Vec2Model { x, y });
wrap_model!(i64, I64Vec3, I64Vec3Model { x, y, z });
wrap_model!(i64, I64Vec4, I64Vec4Model { x, y, z, w });
wrap_model!(u64, U64Vec2, U64Vec2Model { x, y });
wrap_model!(u64, U64Vec3, U64Vec3Model { x, y, z });
wrap_model!(u64, U64Vec4, U64Vec4Model { x, y, z, w });
