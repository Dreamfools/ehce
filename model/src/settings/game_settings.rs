use bevy_reflect::Reflect;
use common_model::color::ColorModel;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
#[serde(deny_unknown_fields)]
pub struct GameSettings {
    pub zoom: f32,
    pub draw_grid: bool,
    pub grid_color: ColorModel,
    pub grid_scale: f32,
}
