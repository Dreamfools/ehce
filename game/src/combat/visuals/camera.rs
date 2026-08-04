use crate::combat::CombatPostUpdate;
use crate::combat::settings::CombatSettings;
use crate::state::GameState;
use bevy::app::{App, Plugin, Update};
use bevy::ecs::query::QuerySingleError;
use bevy::math::{Rect, Vec2, Vec3, Vec3Swizzles as _};
use bevy::prelude::{
    ChildOf, Component, If, IntoScheduleConfigs as _, Query, Reflect, Res, Transform, With,
    Without, error, in_state,
};
use bevy::time::Time;
use bevy::window::{PrimaryWindow, Window};
use inline_tweak::tweak;
use utils::cmp::{max, min};
use utils::decay::exp_decay;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (sys_update_camera_targets, sys_update_camera_transform)
                .chain()
                .run_if(in_state(GameState::Gameplay)),
        )
        .add_systems(CombatPostUpdate, sys_update_world_wrapping);
    }
}

#[derive(Debug, Clone, Default, Reflect, Component)]
pub struct CombatCamera {
    target: Vec2,
    secondary_target: Vec2,
    target_offset: Vec2,
}

#[derive(Debug, Clone, Reflect, Component)]
pub struct CombatBackground;

#[derive(Debug, Clone, Reflect, Component)]
pub struct CameraFollowTarget;

#[derive(Debug, Clone, Reflect, Component)]
pub struct CameraFollowSecondaryTarget;

#[derive(Debug, Clone, Reflect, Component)]
pub struct WorldWrapping;

#[derive(Debug, Clone, Reflect, Component)]
pub struct WorldWrappingCenter;

fn sys_update_camera_targets(
    cam_q: Query<&mut CombatCamera>,
    target_q: Query<&Transform, With<CameraFollowTarget>>,
    secondary_target_q: Query<
        &Transform,
        (
            With<CameraFollowSecondaryTarget>,
            Without<CameraFollowTarget>,
        ),
    >,
) {
    let mut centers_sum = Vec2::ZERO;
    let mut centers_count = 0;

    for t in target_q.iter() {
        let xy = t.translation.xy();
        centers_count += 1;
        centers_sum += xy;
    }
    let center = if centers_count > 0 {
        centers_sum / centers_count as f32
    } else {
        Vec2::ZERO
    };

    let mut targets_sum = Vec2::ZERO;
    let mut targets_count = 0;

    for t in secondary_target_q.iter() {
        let xy = t.translation.xy();
        targets_count += 1;
        targets_sum += xy;
    }

    let targets_avg = if targets_count > 0 {
        targets_sum / targets_count as f32
    } else {
        center
    };

    for mut cam in cam_q {
        cam.target = center;
        cam.secondary_target = targets_avg;
    }
}

fn sys_update_camera_transform(
    mut cam_q: Query<(&mut Transform, &mut CombatCamera)>,
    settings: If<Res<CombatSettings>>,
    time: Res<Time>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    let mut aspect_ratio = 1.0;
    if let Ok(window) = window.single() {
        aspect_ratio = window.width() / window.height();
    }
    for (mut cam_transform, mut cam) in cam_q.iter_mut() {
        let center = cam.target;
        let secondary = cam.secondary_target;

        let offset = secondary - center;

        let mut min_rect = Vec2::splat(settings.min_view_rect_size);
        let mut max_rect = Vec2::splat(
            min(settings.battlefield_size.x, settings.battlefield_size.y) / 4.0
                - settings.view_rect_margins,
        );

        let mut rect_size = offset.abs();
        let rect_ar = rect_size.x / rect_size.y;
        if aspect_ratio > 1.0 {
            min_rect.y /= aspect_ratio;
            max_rect.y /= aspect_ratio;
        } else {
            min_rect.x *= aspect_ratio;
            max_rect.x *= aspect_ratio;
        }

        if rect_ar > aspect_ratio {
            rect_size.y = rect_size.x / aspect_ratio;
        } else {
            rect_size.x = rect_size.y * aspect_ratio;
        }

        rect_size.x = rect_size.x.clamp(min_rect.x, max_rect.x);
        rect_size.y = rect_size.y.clamp(min_rect.y, max_rect.y);

        let zoom_factor = if aspect_ratio > 1.0 {
            (rect_size.x + settings.view_rect_margins) / 64.0
        } else {
            (rect_size.y + settings.view_rect_margins) / 64.0
        };

        let downscale_factor = max(offset.x.abs() / max_rect.x, offset.y.abs() / max_rect.y);
        let adjusted_offset = if downscale_factor > 1.0 {
            offset / downscale_factor
        } else {
            offset
        };

        let new_center = center + adjusted_offset / 2.0;

        let new_position = Vec2::new(new_center.x, new_center.y);
        let decay_factor = tweak!(8.0);

        cam.target_offset = exp_decay(
            cam.target_offset,
            new_position - center,
            decay_factor,
            time.delta_secs(),
        );

        let new_pos = center + cam.target_offset;
        cam_transform.translation = Vec3::new(new_pos.x, new_pos.y, cam_transform.translation.z);
        cam_transform.scale = exp_decay(
            cam_transform.scale,
            Vec3::splat(zoom_factor),
            decay_factor,
            time.delta_secs(),
        );
        // cam_transform.translation = new_position;
        // cam_transform.scale = Vec3::splat(zoom_factor);
    }
}

fn sys_update_world_wrapping(
    q: Query<
        &mut Transform,
        (
            With<WorldWrapping>,
            Without<ChildOf>,
            Without<WorldWrappingCenter>,
        ),
    >,
    mut center_q: Query<&mut Transform, With<WorldWrappingCenter>>,
    settings: If<Res<CombatSettings>>,
) {
    let width = settings.battlefield_size.x;
    let height = settings.battlefield_size.y;
    let half_width = width / 2.0;
    let half_height = height / 2.0;

    let center = match center_q.single_mut() {
        Ok(mut center) => {
            wrap(
                &mut center.translation.x,
                -width,
                width,
                0.0,
                width * 2.0,
                width,
            );
            wrap(
                &mut center.translation.y,
                -height,
                height,
                0.0,
                height * 2.0,
                height,
            );
            center.translation.xy()
        }
        Err(QuerySingleError::NoEntities(_)) => {
            // no center defined, no wrapping
            Vec2::ZERO
        }
        Err(QuerySingleError::MultipleEntities(_)) => {
            error!("Multiple entities are defined as transform center");
            Vec2::ZERO
        }
    };
    let rect = Rect::from_center_size(center, settings.battlefield_size);

    for mut t in q {
        wrap(
            &mut t.translation.x,
            rect.min.x,
            rect.max.x,
            center.x,
            width,
            half_width,
        );
        wrap(
            &mut t.translation.y,
            rect.min.y,
            rect.max.y,
            center.y,
            height,
            half_height,
        );
    }
}
// #[inline]
// fn wrap(x: &mut f32, min: f32, max: f32, center: f32, size: f32, half_size: f32) {
//     if *x < min || *x > max {
//         let dx = *x - center;
//         let off = half_size.copysign(dx);
//         *x = (dx + off) % size - off
//     }
// }

#[inline]
fn wrap(x: &mut f32, min_x: f32, max_x: f32, _: f32, width: f32, _: f32) {
    if *x < min_x || *x > max_x {
        *x = (*x - min_x).rem_euclid(width) + min_x;
    }
}
