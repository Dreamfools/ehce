use crate::hardcoded_constants::SPACE_BACKGROUND_SHADER;
use bevy::math::Vec4;
use bevy::prelude::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::Material2d;
use bevy_asset::Asset;

#[derive(Debug, Clone, Asset, TypePath, AsBindGroup)]
pub struct BackgroundMaterial {
    #[uniform(0)]
    pub transform: Vec4,
}

impl Material2d for BackgroundMaterial {
    fn fragment_shader() -> ShaderRef {
        SPACE_BACKGROUND_SHADER.into()
    }
}
