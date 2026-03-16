use crate::numbers::validated::ValueValidator;
use bevy_reflect::TypePath;
use num_traits::NumCast;
use schemars::Schema;
use serde_json::Value;
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::Bound;

#[derive(TypePath)]
pub struct InRange<R>(PhantomData<fn() -> R>);

#[derive(TypePath)]
pub struct ConstRangeInclusive<const MIN: i64, const MAX: i64>;
pub type InConstRangeInclusive<const MIN: i64, const MAX: i64> =
    InRange<ConstRangeInclusive<MIN, MAX>>;

impl<const MIN: i64, const MAX: i64> ValidateRange for ConstRangeInclusive<MIN, MAX> {
    fn get_range() -> (Bound<f64>, Bound<f64>) {
        (Bound::Included(MIN as f64), Bound::Included(MAX as f64))
    }
}

#[derive(TypePath)]
pub struct ConstOpenInclusiveMinRange<const MIN: i64>;
pub type AtLeast<const MIN: i64> = InRange<ConstOpenInclusiveMinRange<MIN>>;

impl<const MIN: i64> ValidateRange for ConstOpenInclusiveMinRange<MIN> {
    fn get_range() -> (Bound<f64>, Bound<f64>) {
        (Bound::Included(MIN as f64), Bound::Unbounded)
    }
}

#[derive(TypePath)]
pub struct ConstOpenExclusiveMinRange<const MIN: i64>;
pub type AtLeastExclusive<const MIN: i64> = InRange<ConstOpenExclusiveMinRange<MIN>>;

impl<const MIN: i64> ValidateRange for ConstOpenExclusiveMinRange<MIN> {
    fn get_range() -> (Bound<f64>, Bound<f64>) {
        (Bound::Excluded(MIN as f64), Bound::Unbounded)
    }
}

pub trait ValidateRange: TypePath {
    fn get_range() -> (Bound<f64>, Bound<f64>);

    #[must_use]
    fn format_range() -> String {
        let range = Self::get_range();
        let mut w = String::new();
        match range.0 {
            Bound::Included(min) => w.push_str(&format!("[{}, ", min)),
            Bound::Excluded(min) => w.push_str(&format!("({}, ", min)),
            Bound::Unbounded => w.push_str("(-∞, "),
        }
        match range.1 {
            Bound::Included(max) => w.push_str(&format!("{}]", max)),
            Bound::Excluded(max) => w.push_str(&format!("{})", max)),
            Bound::Unbounded => w.push_str("∞)"),
        }
        w
    }
}

impl<T: Copy + NumCast + Display, R: ValidateRange> ValueValidator<T> for InRange<R> {
    fn validate(value: &T) -> Result<(), String> {
        let (min, max) = R::get_range();

        match min {
            Bound::Included(min) => {
                let min_cast = num_traits::cast::<T, f64>(*value)
                    .expect("Failed to cast value to range item type");
                if min_cast < min {
                    return Err(format!(
                        "Value must be at least {}, but got: {}",
                        min, value
                    ));
                }
            }
            Bound::Excluded(min) => {
                let min_cast = num_traits::cast::<T, f64>(*value)
                    .expect("Failed to cast value to range item type");
                if min_cast <= min {
                    return Err(format!(
                        "Value must be greater than {}, but got: {}",
                        min, value
                    ));
                }
            }
            Bound::Unbounded => {}
        }

        match max {
            Bound::Included(max) => {
                let max_cast = num_traits::cast::<T, f64>(*value)
                    .expect("Failed to cast value to range item type");
                if max_cast > max {
                    return Err(format!("Value must be at most {}, but got: {}", max, value));
                }
            }
            Bound::Excluded(max) => {
                let max_cast = num_traits::cast::<T, f64>(*value)
                    .expect("Failed to cast value to range item type");
                if max_cast >= max {
                    return Err(format!(
                        "Value must be less than {}, but got: {}",
                        max, value
                    ));
                }
            }
            Bound::Unbounded => {}
        }

        Ok(())
    }

    fn debug() -> std::borrow::Cow<'static, str> {
        format!("InRange<{}>", R::format_range()).into()
    }

    fn modify_schema(schema: &mut Schema) {
        match schema.get("type") {
            Some(Value::String(ty)) if ty == "number" || ty == "integer" => {
                let (min, max) = R::get_range();

                match min {
                    Bound::Included(min) => {
                        schema.insert(
                            "minimum".to_string(),
                            Value::Number(
                                serde_json::Number::from_f64(min)
                                    .expect("bound if a finite non-nan value"),
                            ),
                        );
                    }
                    Bound::Excluded(min) => {
                        schema.insert(
                            "exclusiveMinimum".to_string(),
                            Value::Number(
                                serde_json::Number::from_f64(min)
                                    .expect("bound if a finite non-nan value"),
                            ),
                        );
                    }
                    Bound::Unbounded => {}
                }

                match max {
                    Bound::Included(max) => {
                        schema.insert(
                            "maximum".to_string(),
                            Value::Number(
                                serde_json::Number::from_f64(max)
                                    .expect("bound if a finite non-nan value"),
                            ),
                        );
                    }
                    Bound::Excluded(max) => {
                        schema.insert(
                            "exclusiveMaximum".to_string(),
                            Value::Number(
                                serde_json::Number::from_f64(max)
                                    .expect("bound if a finite non-nan value"),
                            ),
                        );
                    }
                    Bound::Unbounded => {}
                }
            }
            _ => {}
        }
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        format!("InRange{}", R::format_range()).into()
    }
}
