use bevy_reflect::Reflect;
use num_traits::{PrimInt, Unsigned};
use schemars::_private::serde_json::Value;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Cow;
use std::fmt;
use std::fmt::{Debug, Display};
use std::ops::Deref;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Value must be between {} and {}, but got: {}", .1, .2, .0)]
pub struct BadValueError(u64, u64, u64);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Reflect)]
#[serde(transparent)]
#[repr(transparent)]
pub struct UIntInRange<T: PrimInt + Unsigned, const MIN: u64, const MAX: u64>(T);

impl<T: PrimInt + Unsigned, const MIN: u64, const MAX: u64> UIntInRange<T, MIN, MAX> {
    #[inline]
    pub fn try_new(value: T) -> Result<Self, BadValueError> {
        let value_u64: u64 = value.to_u64().unwrap();
        if value_u64 >= MIN && value_u64 <= MAX {
            Ok(Self(value))
        } else {
            Err(BadValueError(value_u64, MIN, MAX))
        }
    }

    #[inline]
    #[must_use]
    pub fn min() -> Self {
        Self(T::from(MIN).unwrap())
    }

    #[inline]
    #[must_use]
    pub fn max() -> Self {
        Self(T::from(MAX).unwrap())
    }

    #[inline]
    pub fn get(&self) -> T {
        self.0
    }
}

impl<T: PrimInt + Unsigned, const MIN: u64, const MAX: u64> Default for UIntInRange<T, MIN, MAX> {
    #[inline]
    fn default() -> Self {
        Self(T::from(MIN).unwrap())
    }
}

impl<T: PrimInt + Unsigned, const MIN: u64, const MAX: u64> Deref for UIntInRange<T, MIN, MAX> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: PrimInt + Unsigned + Display, const MIN: u64, const MAX: u64> Display
    for UIntInRange<T, MIN, MAX>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de, T: PrimInt + Unsigned + Debug + Deserialize<'de>, const MIN: u64, const MAX: u64>
    Deserialize<'de> for UIntInRange<T, MIN, MAX>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = T::deserialize(deserializer)?;
        Self::try_new(inner).map_err(serde::de::Error::custom)
    }
}

impl<T: PrimInt + Unsigned + Debug + JsonSchema, const MIN: u64, const MAX: u64> JsonSchema
    for UIntInRange<T, MIN, MAX>
{
    fn inline_schema() -> bool {
        T::inline_schema()
    }

    fn schema_name() -> Cow<'static, str> {
        "IntInRange".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let mut schema = T::json_schema(generator);
        match schema.get("type") {
            Some(Value::String(ty)) if ty == "number" || ty == "integer" => {
                schema.insert("minimum".to_string(), Value::Number(MIN.into()));
                schema.insert("maximum".to_string(), Value::Number(MAX.into()));
                schema
            }
            _ => schema,
        }
    }
}
