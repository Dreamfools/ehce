use bevy_reflect::Reflect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use common_model::color::ColorModel;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Reflect)]
pub struct GameSettings {
    pub zoom: f32,
    pub draw_grid: bool,
    pub grid_color: ColorModel,
    pub grid_scale: f32,
}