use bevy_reflect::{Reflect, TypePath};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::Serialize;
use std::borrow::Cow;
use std::fmt::Debug;
use std::marker::PhantomData;

pub mod common;
pub mod in_range;
pub mod is_finite;
pub mod variadic;

#[derive(Reflect)]
#[repr(transparent)]
pub struct Validated<T, V: ValueValidator<T>> {
    value: T,
    #[reflect(ignore)]
    validator: PhantomData<fn() -> V>,
}

impl<T, V: ValueValidator<T>> Validated<T, V> {
    pub fn new(value: T) -> Result<Self, String> {
        V::validate(&value)?;
        Ok(Self {
            value,
            validator: PhantomData,
        })
    }

    pub fn value(&self) -> &T {
        &self.value
    }
}

pub trait ValueValidator<T>: TypePath {
    /// Validates the value and returns an error message if it's invalid
    fn validate(value: &T) -> Result<(), String>;

    /// Returns a debug string for the validator, used in the `Debug`
    /// implementation of `Validated`
    #[must_use]
    fn debug() -> Cow<'static, str> {
        std::any::type_name::<T>().into()
    }

    /// Modifies the JSON schema for the validated type, e.g. by adding
    /// constraints
    fn modify_schema(schema: &mut Schema) {
        let _ = schema;
    }

    /// Returns a name for the schema of the validated type, used in the
    /// `JsonSchema` implementation of `Validated`
    #[must_use]
    fn schema_name() -> Cow<'static, str> {
        Self::debug()
    }
}

const _: () = {
    impl<T, V: ValueValidator<T>> Debug for Validated<T, V>
    where
        T: Debug,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_tuple("Validated")
                .field(&self.value)
                .field(&V::debug())
                .finish()
        }
    }

    impl<T, V: ValueValidator<T>> Clone for Validated<T, V>
    where
        T: Clone,
    {
        fn clone(&self) -> Self {
            Self {
                value: self.value.clone(),
                validator: PhantomData,
            }
        }
    }

    impl<T, V: ValueValidator<T>> Copy for Validated<T, V> where T: Copy {}

    impl<T: PartialEq, V: ValueValidator<T>> PartialEq<Self> for Validated<T, V> {
        fn eq(&self, other: &Self) -> bool {
            self.value == other.value
        }
    }

    impl<T: Eq, V: ValueValidator<T>> Eq for Validated<T, V> {}

    impl<T: PartialOrd, V: ValueValidator<T>> PartialOrd<Self> for Validated<T, V> {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            self.value.partial_cmp(&other.value)
        }
    }

    impl<T: Ord, V: ValueValidator<T>> Ord for Validated<T, V> {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.value.cmp(&other.value)
        }
    }

    impl<T: std::hash::Hash, V: ValueValidator<T>> std::hash::Hash for Validated<T, V> {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.value.hash(state);
        }
    }

    impl<T, V: ValueValidator<T>> std::ops::Deref for Validated<T, V> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.value
        }
    }

    impl<T, V: ValueValidator<T>> Serialize for Validated<T, V>
    where
        T: Serialize,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.value.serialize(serializer)
        }
    }

    impl<'de, T, V: ValueValidator<T>> serde::Deserialize<'de> for Validated<T, V>
    where
        T: serde::Deserialize<'de>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let value = T::deserialize(deserializer)?;
            Self::new(value).map_err(serde::de::Error::custom)
        }
    }

    impl<T, V: ValueValidator<T>> JsonSchema for Validated<T, V>
    where
        T: JsonSchema,
    {
        fn schema_name() -> Cow<'static, str> {
            format!("Validated<{}, {}>", T::schema_name(), V::schema_name()).into()
        }

        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            let mut schema = T::json_schema(generator);
            V::modify_schema(&mut schema);
            schema
        }

        fn inline_schema() -> bool {
            T::inline_schema()
        }
    }
};
