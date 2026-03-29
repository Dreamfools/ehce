use crate::numbers::validated::ValueValidator;
use bevy_reflect::TypePath;
use num_traits::Float;
use std::borrow::Cow;
use std::fmt::Display;

#[derive(TypePath)]
pub struct IsFinite;

impl<T: Float + Display> ValueValidator<T> for IsFinite {
    fn validate(value: &T) -> Result<(), String> {
        if value.is_finite() {
            Ok(())
        } else {
            Err(format!("Value {} is not finite", value))
        }
    }

    fn debug() -> Cow<'static, str> {
        "IsFinite".into()
    }
}
