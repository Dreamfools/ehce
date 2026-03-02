use bevy_color::ColorToPacked;
use bevy_reflect::Reflect;
use float_ord::FloatOrd;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;

#[derive(Copy, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
pub struct ColorModel(
    #[serde(with = "utils::serde_with::color")]
    #[schemars(with = "utils::serde_with::color::ColorDataSerialized")]
    pub bevy_color::LinearRgba,
);

#[inline]
#[must_use]
pub const fn default_white() -> ColorModel {
    ColorModel(bevy_color::LinearRgba::WHITE)
}

#[inline]
#[must_use]
pub fn is_default_white(c: &ColorModel) -> bool {
    c.0 == bevy_color::LinearRgba::WHITE
}

impl Hash for ColorModel {
    fn hash<H: Hasher>(&self, state: &mut H) {
        FloatOrd(self.red).hash(state);
        FloatOrd(self.green).hash(state);
        FloatOrd(self.blue).hash(state);
        FloatOrd(self.alpha).hash(state);
    }
}

impl PartialEq for ColorModel {
    fn eq(&self, other: &Self) -> bool {
        FloatOrd(self.red) == FloatOrd(other.red)
            && FloatOrd(self.green) == FloatOrd(other.green)
            && FloatOrd(self.blue) == FloatOrd(other.blue)
            && FloatOrd(self.alpha) == FloatOrd(other.alpha)
    }
}

impl Eq for ColorModel {}

impl PartialOrd for ColorModel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ColorModel {
    fn cmp(&self, other: &Self) -> Ordering {
        FloatOrd(self.red)
            .cmp(&FloatOrd(other.red))
            .then_with(|| FloatOrd(self.green).cmp(&FloatOrd(other.green)))
            .then_with(|| FloatOrd(self.blue).cmp(&FloatOrd(other.blue)))
            .then_with(|| FloatOrd(self.alpha).cmp(&FloatOrd(other.alpha)))
    }
}

impl Debug for ColorModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let rgba: [u8; 4] = self.0.to_u8_array();
        if self.alpha == 1.0 {
            write!(f, "#{:02x}{:02x}{:02x}", rgba[0], rgba[1], rgba[2])
        } else {
            write!(
                f,
                "RGBA#{:02x}{:02x}{:02x}{:02x}",
                rgba[0], rgba[1], rgba[2], rgba[3]
            )
        }
    }
}

impl Deref for ColorModel {
    type Target = bevy_color::LinearRgba;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
