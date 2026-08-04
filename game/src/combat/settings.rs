use bevy::math::Vec2;
use bevy::prelude::{Reflect, Resource};

#[derive(Debug, Clone, Reflect, Resource)]
pub struct CombatSettings {
    /// Size of the battlefield in world units
    pub battlefield_size: Vec2,
    pub min_view_rect_size: f32,
    pub view_rect_margins: f32,
}

impl Default for CombatSettings {
    fn default() -> Self {
        Self {
            battlefield_size: Vec2::new(200.0, 200.0),
            min_view_rect_size: 16.0,
            view_rect_margins: 8.0,
        }
    }
}
