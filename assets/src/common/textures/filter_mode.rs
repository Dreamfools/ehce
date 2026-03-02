use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Facet)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum FilterMode {
    #[default]
    Linear = 0,
    Nearest = 1,
}

impl FilterMode {
    #[must_use] pub fn to_macroquad(self) -> macroquad::texture::FilterMode {
        match self {
            FilterMode::Nearest => macroquad::texture::FilterMode::Nearest,
            FilterMode::Linear => macroquad::texture::FilterMode::Linear,
        }
    }
    
    #[must_use] pub fn to_yakui(self) -> yakui::paint::TextureFilter {
        match self {
            FilterMode::Nearest => yakui::paint::TextureFilter::Nearest,
            FilterMode::Linear => yakui::paint::TextureFilter::Linear,
        }
    }
}