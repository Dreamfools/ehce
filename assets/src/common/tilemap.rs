use facet::Facet;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use common_model::numbers::glam_wraps::UVec2Model;
use crate::common::textures::SpriteId;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Facet)]
pub struct Tilemap {
    pub sprite: SpriteId,
    pub tile_size: UVec2Model,
    #[serde(default)]
    pub offset: UVec2Model,
    #[serde(default)]
    pub gap: UVec2Model,
}