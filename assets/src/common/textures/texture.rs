use facet::Facet;
use serde::{Deserialize, Serialize};
use crate::common::textures::filter_mode::FilterMode;

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Facet)]
pub struct TextureMetadata {
    pub filter_mode: FilterMode
}