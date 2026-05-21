use std::time::Instant;

use gpui::{
    Bounds, Hsla, PathBuilder, Pixels, Point, Window, linear_color_stop, linear_gradient, point,
    px, size,
};

use crate::{CursorShape, editor_settings::SmoothCursorSettings};

const SMOOTH_CURSOR_SETTLE_DISTANCE_PX: f32 = 0.35;
const TRAIL_MIN_PEAK_DISTANCE_PX: f32 = 1.0;
const TRAIL_HALO_MAX_RADIUS_PX: f32 = 6.0;

/// Corner-based cursor trail animation inspired by Kitty's `cursor_trail.c`.
///
/// The editor keeps the logical cursor exact, while this state only controls
/// the visual interpolation of the local painted cursor.
#[derive(Clone, Debug)]
pub(crate) struct SmoothCursorAnimationState {
    target_bounds: Bounds<Pixels>,
    corners: [gpui::Point<Pixels>; 4],
    peak_distance: f32,
    last_frame: Instant,
}

pub(crate) struct SmoothCursorFrame {
    pub(crate) trail: Option<SmoothCursorTrail>,
    pub(crate) animating: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct SmoothCursorTrail {
    outer_points: [Point<Pixels>; 6],
    inner_points: [Point<Pixels>; 6],
    halo_points: [Point<Pixels>; 6],
    tail_color: Hsla,
    head_color: Hsla,
    core_color: Hsla,
    gradient_angle: f32,
    body_alpha: f32,
    halo_alpha: f32,
    core_alpha: f32,
}

impl SmoothCursorAnimationState {
    pub(crate) fn new(bounds: Bounds<Pixels>, now: Instant) -> Self {
        Self {
            target_bounds: bounds,
            corners: bounds_corners(bounds),
            peak_distance: 0.0,
            last_frame: now,
        }
    }

    pub(crate) fn snap_to(&mut self, bounds: Bounds<Pixels>, now: Instant) {
        self.target_bounds = bounds;
        self.corners = bounds_corners(bounds);
        self.peak_distance = 0.0;
        self.last_frame = now;
    }

    pub(crate) fn retarget(&mut self, bounds: Bounds<Pixels>) {
        // We intentionally do not impose a distance snap here. Long jumps such
        // as `gg` should leave a continuous trail, which is the main part of
        // the Kitty-inspired effect.
        if self.target_bounds != bounds {
            let target_corners = bounds_corners(bounds);
            self.peak_distance =
                max_corner_distance(&self.corners, &target_corners).max(TRAIL_MIN_PEAK_DISTANCE_PX);
        }
        self.target_bounds = bounds;
    }

    pub(crate) fn step(
        &mut self,
        now: Instant,
        color: Hsla,
        settings: &SmoothCursorSettings,
    ) -> SmoothCursorFrame {
        let dt = now
            .duration_since(self.last_frame)
            .as_secs_f32()
            .clamp(1.0 / 240.0, 1.0 / 24.0);
        self.last_frame = now;

        let target_corners = bounds_corners(self.target_bounds);
        let center_x = self.target_bounds.center().x;
        let center_y = self.target_bounds.center().y;
        let half_diagonal = ((self.target_bounds.size.width.as_f32().powi(2)
            + self.target_bounds.size.height.as_f32().powi(2))
        .sqrt()
            / 2.0)
            .max(0.001);

        let mut dx = [0.0; 4];
        let mut dy = [0.0; 4];
        let mut dot = [0.0; 4];
        let mut animating = false;

        for i in 0..4 {
            dx[i] = target_corners[i].x.as_f32() - self.corners[i].x.as_f32();
            dy[i] = target_corners[i].y.as_f32() - self.corners[i].y.as_f32();
            let dist = (dx[i] * dx[i] + dy[i] * dy[i]).sqrt();

            if dist > SMOOTH_CURSOR_SETTLE_DISTANCE_PX {
                animating = true;
                let corner_to_center_x = target_corners[i].x.as_f32() - center_x.as_f32();
                let corner_to_center_y = target_corners[i].y.as_f32() - center_y.as_f32();
                dot[i] = (dx[i] * corner_to_center_x + dy[i] * corner_to_center_y)
                    / (half_diagonal * dist.max(0.001));
            } else {
                self.corners[i] = target_corners[i];
                dx[i] = 0.0;
                dy[i] = 0.0;
                dot[i] = 0.0;
            }
        }

        if animating {
            let min_dot = dot.iter().copied().fold(f32::MAX, f32::min);
            let max_dot = dot.iter().copied().fold(f32::MIN, f32::max);
            let decay_fast = settings.leading_smooth_time.as_secs_f32().clamp(0.01, 2.0);
            let decay_slow = settings.smooth_time.as_secs_f32().clamp(0.04, 2.0);

            for i in 0..4 {
                if dx[i] == 0.0 && dy[i] == 0.0 {
                    continue;
                }

                let decay = if (max_dot - min_dot).abs() < 1e-5 {
                    decay_slow
                } else {
                    decay_slow
                        + (decay_fast - decay_slow) * (dot[i] - min_dot) / (max_dot - min_dot)
                };

                let step = 1.0 - (-10.0 * dt / decay).exp2();
                self.corners[i].x += px(dx[i] * step);
                self.corners[i].y += px(dy[i] * step);
            }
        }

        let remaining_distance = max_corner_distance(&self.corners, &target_corners);
        let trail = if settings.trail && settings.trail_opacity > 0.0 && animating {
            if remaining_distance > settings.trail_min_distance {
                let target_center = self.target_bounds.center();
                let animated_center = quad_center(&self.corners);
                let motion = point(
                    target_center.x - animated_center.x,
                    target_center.y - animated_center.y,
                );
                let motion_distance =
                    (motion.x.as_f32().powi(2) + motion.y.as_f32().powi(2)).sqrt();
                let progress = if self.peak_distance > TRAIL_MIN_PEAK_DISTANCE_PX {
                    1.0 - (remaining_distance / self.peak_distance).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                let time_fade = 1.0 - progress * progress;
                let cursor_extent = self
                    .target_bounds
                    .size
                    .width
                    .as_f32()
                    .max(self.target_bounds.size.height.as_f32())
                    .max(1.0);
                let distance_fade = (remaining_distance / (cursor_extent * 0.55).max(1.0))
                    .clamp(0.0, 1.0)
                    .sqrt();
                let motion_fade = (motion_distance / (cursor_extent * 0.45).max(1.0))
                    .clamp(0.0, 1.0)
                    .sqrt();
                let spatial_fade = distance_fade.max(motion_fade * 0.35);
                let body_alpha = settings.trail_opacity.clamp(0.0, 1.0) * spatial_fade * time_fade;

                if body_alpha > 0.01 && motion_distance > settings.trail_min_distance {
                    let selector = (
                        if motion.x.as_f32() >= 0.0 { 1.0 } else { 0.0 },
                        if motion.y.as_f32() >= 0.0 { 1.0 } else { 0.0 },
                    );
                    let current_quad = TrailQuad::from_corners(target_corners);
                    let lagging_quad = TrailQuad::from_corners(self.corners);

                    let (curr_p1, curr_p2, curr_p3, curr_p4) =
                        select_corners(current_quad, selector);
                    let (lag_p1, lag_p2, lag_p3) = select_trail_corners(lagging_quad, selector);

                    let head_mix = ease_clamped(progress);
                    let side_mix = ease_clamped((progress * 2.0).clamp(0.0, 1.0));

                    let outer_points = [
                        mix_points(lag_p1, curr_p1, head_mix),
                        mix_points(lag_p2, curr_p2, side_mix),
                        curr_p2,
                        curr_p4,
                        curr_p3,
                        mix_points(lag_p3, curr_p3, side_mix),
                    ];

                    let halo_radius = (self.target_bounds.size.height.as_f32() * 0.32)
                        .clamp(1.5, TRAIL_HALO_MAX_RADIUS_PX);
                    let polygon_center = polygon_center(&outer_points);
                    let halo_points = scale_polygon(
                        &outer_points,
                        polygon_center,
                        1.0 + halo_radius / motion_distance.max(halo_radius),
                    );
                    let inner_points = scale_polygon(
                        &outer_points,
                        target_center,
                        (1.0 - 0.18 * spatial_fade).clamp(0.72, 0.92),
                    );

                    let tail_color = Hsla {
                        h: color.h,
                        s: (color.s * 0.88).clamp(0.0, 1.0),
                        l: (color.l * 0.72).clamp(0.0, 0.9),
                        a: 1.0,
                    };
                    let head_color = Hsla {
                        h: color.h,
                        s: (color.s * 0.9 + 0.08).clamp(0.0, 1.0),
                        l: (color.l + 0.16).clamp(0.0, 0.98),
                        a: 1.0,
                    };
                    let core_color = Hsla {
                        h: color.h,
                        s: (color.s * 0.2).clamp(0.0, 0.4),
                        l: 0.98,
                        a: 1.0,
                    };

                    Some(SmoothCursorTrail {
                        outer_points,
                        inner_points,
                        halo_points,
                        tail_color,
                        head_color,
                        core_color,
                        gradient_angle: gradient_angle(motion),
                        body_alpha,
                        halo_alpha: body_alpha * 0.45,
                        core_alpha: body_alpha * (0.45 + 0.35 * spatial_fade),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if !animating {
            self.peak_distance = 0.0;
        }

        SmoothCursorFrame { trail, animating }
    }
}

impl SmoothCursorTrail {
    pub(crate) fn paint(&self, origin: gpui::Point<Pixels>, window: &mut Window) {
        let halo_points = self.halo_points.map(|point| point + origin);
        paint_polygon(
            &halo_points,
            linear_gradient(
                self.gradient_angle,
                linear_color_stop(self.tail_color.opacity(self.halo_alpha * 0.12), 0.0),
                linear_color_stop(self.head_color.opacity(self.halo_alpha), 1.0),
            ),
            window,
        );

        let outer_points = self.outer_points.map(|point| point + origin);
        paint_polygon(
            &outer_points,
            linear_gradient(
                self.gradient_angle,
                linear_color_stop(self.tail_color.opacity(self.body_alpha * 0.45), 0.0),
                linear_color_stop(self.head_color.opacity(self.body_alpha), 1.0),
            ),
            window,
        );

        let inner_points = self.inner_points.map(|point| point + origin);
        paint_polygon(
            &inner_points,
            linear_gradient(
                self.gradient_angle,
                linear_color_stop(self.head_color.opacity(0.0), 0.45),
                linear_color_stop(self.core_color.opacity(self.core_alpha), 1.0),
            ),
            window,
        );
    }
}

pub(crate) fn cursor_bounds(
    origin: gpui::Point<Pixels>,
    block_width: Pixels,
    line_height: Pixels,
    shape: CursorShape,
) -> Bounds<Pixels> {
    match shape {
        CursorShape::Bar => Bounds {
            origin,
            size: size(px(2.0), line_height),
        },
        CursorShape::Block | CursorShape::Hollow => Bounds {
            origin,
            size: size(block_width, line_height),
        },
        CursorShape::Underline => Bounds {
            origin: origin + gpui::Point::new(Pixels::ZERO, line_height - px(2.0)),
            size: size(block_width, px(2.0)),
        },
    }
}

fn bounds_corners(bounds: Bounds<Pixels>) -> [gpui::Point<Pixels>; 4] {
    [
        point(bounds.left(), bounds.top()),
        point(bounds.right(), bounds.top()),
        point(bounds.right(), bounds.bottom()),
        point(bounds.left(), bounds.bottom()),
    ]
}

#[derive(Clone, Copy, Debug)]
struct TrailQuad {
    top_left: Point<Pixels>,
    top_right: Point<Pixels>,
    bottom_left: Point<Pixels>,
    bottom_right: Point<Pixels>,
}

impl TrailQuad {
    fn from_corners(corners: [Point<Pixels>; 4]) -> Self {
        Self {
            top_left: corners[0],
            top_right: corners[1],
            bottom_right: corners[2],
            bottom_left: corners[3],
        }
    }
}

fn max_corner_distance(a: &[Point<Pixels>; 4], b: &[Point<Pixels>; 4]) -> f32 {
    (0..4)
        .map(|i| {
            let dx = b[i].x.as_f32() - a[i].x.as_f32();
            let dy = b[i].y.as_f32() - a[i].y.as_f32();
            (dx * dx + dy * dy).sqrt()
        })
        .fold(0.0f32, f32::max)
}

fn quad_center(corners: &[Point<Pixels>; 4]) -> Point<Pixels> {
    let x = corners.iter().map(|corner| corner.x.as_f32()).sum::<f32>() / 4.0;
    let y = corners.iter().map(|corner| corner.y.as_f32()).sum::<f32>() / 4.0;
    point(px(x), px(y))
}

fn polygon_center<const N: usize>(points: &[Point<Pixels>; N]) -> Point<Pixels> {
    let x = points.iter().map(|point| point.x.as_f32()).sum::<f32>() / N as f32;
    let y = points.iter().map(|point| point.y.as_f32()).sum::<f32>() / N as f32;
    point(px(x), px(y))
}

fn mix_points(a: Point<Pixels>, b: Point<Pixels>, t: f32) -> Point<Pixels> {
    point(
        a.x + (b.x - a.x) * t.clamp(0.0, 1.0),
        a.y + (b.y - a.y) * t.clamp(0.0, 1.0),
    )
}

fn scale_polygon<const N: usize>(
    points: &[Point<Pixels>; N],
    center: Point<Pixels>,
    factor: f32,
) -> [Point<Pixels>; N] {
    points.map(|vertex| {
        let dx = vertex.x.as_f32() - center.x.as_f32();
        let dy = vertex.y.as_f32() - center.y.as_f32();
        point(
            px(center.x.as_f32() + dx * factor),
            px(center.y.as_f32() + dy * factor),
        )
    })
}

fn select_trail_corners(
    quad: TrailQuad,
    selector: (f32, f32),
) -> (Point<Pixels>, Point<Pixels>, Point<Pixels>) {
    let (sel_x, sel_y) = selector;
    let p1 = mix_points(
        mix_points(quad.top_right, quad.top_left, sel_x),
        mix_points(quad.bottom_right, quad.bottom_left, sel_x),
        sel_y,
    );
    let p2 = mix_points(
        mix_points(quad.top_left, quad.bottom_left, sel_x),
        mix_points(quad.top_right, quad.bottom_right, sel_x),
        sel_y,
    );
    let p3 = mix_points(
        mix_points(quad.bottom_right, quad.top_right, sel_x),
        mix_points(quad.bottom_left, quad.top_left, sel_x),
        sel_y,
    );
    (p1, p2, p3)
}

fn select_corners(
    quad: TrailQuad,
    selector: (f32, f32),
) -> (Point<Pixels>, Point<Pixels>, Point<Pixels>, Point<Pixels>) {
    let (p1, p2, p3) = select_trail_corners(quad, selector);
    let (sel_x, sel_y) = selector;
    let p4 = mix_points(
        mix_points(quad.bottom_left, quad.bottom_right, sel_x),
        mix_points(quad.top_left, quad.top_right, sel_x),
        sel_y,
    );
    (p1, p2, p3, p4)
}

fn ease_clamped(progress: f32) -> f32 {
    let t = 1.0 - progress.clamp(0.0, 1.0);
    1.0 - t * t * t
}

fn gradient_angle(direction: Point<Pixels>) -> f32 {
    direction
        .x
        .as_f32()
        .atan2(-direction.y.as_f32())
        .to_degrees()
        .rem_euclid(360.0)
}

fn paint_polygon<const N: usize>(
    points: &[Point<Pixels>; N],
    background: impl Into<gpui::Background>,
    window: &mut Window,
) {
    let mut path_builder = PathBuilder::fill();
    path_builder.add_polygon(points, true);
    if let Ok(path) = path_builder.build() {
        window.paint_path(path, background);
    }
}
