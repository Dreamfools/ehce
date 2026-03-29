use crate::numbers::validated::Validated;
use crate::numbers::validated::in_range::{AtLeast, AtLeastExclusive};
use crate::numbers::validated::is_finite::IsFinite;

pub type PositiveFinite<T> = Validated<T, (AtLeastExclusive<0>, IsFinite)>;
pub type NonNegFinite<T> = Validated<T, (AtLeast<0>, IsFinite)>;
