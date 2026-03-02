use num_traits::{Float, Zero};
use schemars::_private::serde_json::Value;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Cow;
use std::fmt;
use std::fmt::{Debug, Display};
use std::ops::{Add, Deref};
use bevy_reflect::Reflect;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Value must be non-negative and finite, but got: {}", .0)]
pub struct BadValueError(f64);

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Serialize, Reflect)]
#[serde(transparent)]
#[repr(transparent)]
pub struct FiniteNonNegative<T: Float>(T);

impl<T: Float + Debug> FiniteNonNegative<T> {
    pub fn try_new(value: T) -> Result<Self, BadValueError> {
        if value >= T::zero() && value.is_finite() {
            Ok(Self(value))
        } else {
            Err(BadValueError(value.to_f64().unwrap()))
        }
    }

    pub fn get(&self) -> T {
        self.0
    }
}

impl<T: Float + Debug> Default for FiniteNonNegative<T> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<T: Float + Debug> Add<Self> for FiniteNonNegative<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let sum = self.0 + rhs.0;
        assert!(sum.is_finite(), "Result of addition is not finite: {sum:?}");
        FiniteNonNegative(sum)
    }
}

impl<T: Float + Debug> Zero for FiniteNonNegative<T> {
    fn zero() -> Self {
        Self(T::zero())
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl<T: Float + Debug> Deref for FiniteNonNegative<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Float + Display> Display for FiniteNonNegative<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de, T: Float + Debug + Deserialize<'de>> Deserialize<'de> for FiniteNonNegative<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = T::deserialize(deserializer)?;
        FiniteNonNegative::try_new(inner).map_err(serde::de::Error::custom)
    }
}

impl<T: Float + Debug + JsonSchema> JsonSchema for FiniteNonNegative<T> {
    fn inline_schema() -> bool {
        T::inline_schema()
    }

    fn schema_name() -> Cow<'static, str> {
        "FiniteNonNegative".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let mut schema = T::json_schema(generator);
        match schema.get("type") {
            Some(Value::String(ty)) if ty == "number" => {
                schema.insert("minimum".to_string(), Value::Number(0.into()));
                schema
            }
            _ => schema,
        }
    }
}
