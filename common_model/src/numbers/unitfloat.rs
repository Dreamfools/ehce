use bevy_reflect::Reflect;
use num_traits::{ConstOne, ConstZero, Float, Zero};
use schemars::_private::serde_json::Value;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Cow;
use std::fmt;
use std::fmt::{Debug, Display};
use std::ops::{Add, Deref};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Value must be between 0 and 1, but got: {}", .0)]
pub struct BadValueError(f64);

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Serialize, Reflect)]
#[serde(transparent)]
#[repr(transparent)]
pub struct ZeroOne<T: Float>(T);

impl<T: Float + Debug> ZeroOne<T> {
    #[inline]
    pub fn try_new(value: T) -> Result<Self, BadValueError> {
        if value.is_finite() && value >= T::zero() && value <= T::one() {
            Ok(Self(value))
        } else {
            Err(BadValueError(value.to_f64().unwrap()))
        }
    }

    #[inline]
    pub fn get(&self) -> T {
        self.0
    }
}

impl<T: Float + Debug + ConstZero> ZeroOne<T> {
    pub const ZERO: Self = Self(T::ZERO);
}

impl<T: Float + Debug + ConstOne> ZeroOne<T> {
    pub const ONE: Self = Self(T::ONE);
}

impl<T: Float + Debug> Default for ZeroOne<T> {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl<T: Float + Debug> Add<Self> for ZeroOne<T> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let sum = self.0 + rhs.0;
        assert!(sum.is_finite(), "Result of addition is not finite: {sum:?}");
        ZeroOne(sum)
    }
}

impl<T: Float + Debug> Zero for ZeroOne<T> {
    #[inline]
    fn zero() -> Self {
        Self(T::zero())
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl<T: Float + Debug> Deref for ZeroOne<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Float + Display> Display for ZeroOne<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de, T: Float + Debug + Deserialize<'de>> Deserialize<'de> for ZeroOne<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = T::deserialize(deserializer)?;
        ZeroOne::try_new(inner).map_err(serde::de::Error::custom)
    }
}

impl<T: Float + Debug + JsonSchema> JsonSchema for ZeroOne<T> {
    fn inline_schema() -> bool {
        T::inline_schema()
    }

    fn schema_name() -> Cow<'static, str> {
        "ZeroOne".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let mut schema = T::json_schema(generator);
        match schema.get("type") {
            Some(Value::String(ty)) if ty == "number" => {
                schema.insert("minimum".to_string(), Value::Number(0.into()));
                schema.insert("maximum".to_string(), Value::Number(1.into()));
                schema
            }
            _ => schema,
        }
    }
}
