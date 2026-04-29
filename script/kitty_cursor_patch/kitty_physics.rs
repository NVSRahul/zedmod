const SMOOTH_CURSOR_SETTLE_DISTANCE_PX: f32 = 0.35;
const SMOOTH_CURSOR_STALE_RESET: Duration = Duration::from_millis(180);
const SMOOTH_CURSOR_RESET_DISTANCE_PX: f32 = 1400.0;

#[derive(Clone, Debug)]
pub(crate) struct SmoothCursorAnimationState {
    target_display_point: DisplayPoint,
    target_bounds: Bounds<Pixels>,
    corners: [gpui::Point<Pixels>; 4],
    shape: CursorShape,
    last_frame: Instant,
}

struct SmoothCursorFrame {
    trail: Option<SmoothCursorTrail>,
    animating: bool,
}

#[derive(Clone, Debug)]
struct SmoothCursorTrail {
    points: [gpui::Point<Pixels>; 4],
    color: Hsla,
}

impl SmoothCursorAnimationState {
    fn new(
        display_point: DisplayPoint,
        bounds: Bounds<Pixels>,
        shape: CursorShape,
        now: Instant,
    ) -> Self {
        Self {
            target_display_point: display_point,
            target_bounds: bounds,
            corners: bounds_corners(bounds),
            shape,
            last_frame: now,
        }
    }

    fn snap_to(
        &mut self,
        display_point: DisplayPoint,
        bounds: Bounds<Pixels>,
        shape: CursorShape,
        now: Instant,
    ) {
        self.target_display_point = display_point;
        self.target_bounds = bounds;
        self.corners = bounds_corners(bounds);
        self.shape = shape;
        self.last_frame = now;
    }

    fn retarget(
        &mut self,
        display_point: DisplayPoint,
        bounds: Bounds<Pixels>,
        shape: CursorShape,
        now: Instant,
    ) {
        let stale = now.duration_since(self.last_frame) > SMOOTH_CURSOR_STALE_RESET;
        let target_delta = point(
            bounds.left() - self.target_bounds.left(),
            bounds.top() - self.target_bounds.top(),
        );
        let reset_distance = ((target_delta.x.as_f32() * target_delta.x.as_f32())
            + (target_delta.y.as_f32() * target_delta.y.as_f32()))
        .sqrt();
        let height_delta = (bounds.size.height - self.target_bounds.size.height).as_f32().abs();

        if stale || reset_distance > SMOOTH_CURSOR_RESET_DISTANCE_PX || height_delta > 1.0 {
            self.snap_to(display_point, bounds, shape, now);
            return;
        }

        self.target_display_point = display_point;
        self.target_bounds = bounds;
        self.shape = shape;
    }

    fn step(
        &mut self,
        now: Instant,
        color: Hsla,
        settings: &crate::editor_settings::SmoothCursorSettings,
    ) -> SmoothCursorFrame {
        let dt = now
            .duration_since(self.last_frame)
            .as_secs_f32()
            .clamp(1.0 / 240.0, 1.0 / 24.0);
        self.last_frame = now;

        let target_corners = bounds_corners(self.target_bounds);
        let center_x = self.target_bounds.center().x;
        let center_y = self.target_bounds.center().y;
        let diag_2 = ((self.target_bounds.size.width.as_f32().powi(2)
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
                    / (diag_2 * dist.max(0.001));
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

            // Kitty's default exponential ease parameters
            let decay_fast = 0.03;
            let decay_slow = settings.smooth_time.as_secs_f32().max(0.04);

            for i in 0..4 {
                if dx[i] == 0.0 && dy[i] == 0.0 {
                    continue;
                }

                let decay = if (max_dot - min_dot).abs() < 1e-5 {
                    decay_slow
                } else {
                    decay_slow + (decay_fast - decay_slow) * (dot[i] - min_dot) / (max_dot - min_dot)
                };

                let step = 1.0 - (-10.0 * dt / decay).exp2();
                self.corners[i].x += px(dx[i] * step);
                self.corners[i].y += px(dy[i] * step);
            }
        }

        let trail = if settings.trail && settings.trail_opacity > 0.0 && animating {
            let max_dist = (0..4)
                .map(|i| {
                    let d_x = target_corners[i].x.as_f32() - self.corners[i].x.as_f32();
                    let d_y = target_corners[i].y.as_f32() - self.corners[i].y.as_f32();
                    (d_x * d_x + d_y * d_y).sqrt()
                })
                .fold(0.0f32, f32::max);

            if max_dist > settings.trail_min_distance {
                let trail_alpha = settings.trail_opacity.clamp(0.0, 1.0)
                    * (max_dist / self.target_bounds.size.height.as_f32().max(1.0)).min(1.0);
                Some(SmoothCursorTrail {
                    points: self.corners,
                    color: color.opacity(trail_alpha),
                })
            } else {
                None
            }
        } else {
            None
        };

        SmoothCursorFrame { trail, animating }
    }
}

impl SmoothCursorTrail {
    fn paint(&self, origin: gpui::Point<Pixels>, window: &mut Window) {
        let points = self
            .points
            .iter()
            .map(|point| *point + origin)
            .collect::<SmallVec<[gpui::Point<Pixels>; 4]>>();
        let mut path_builder = PathBuilder::fill();
        path_builder.add_polygon(&points, true);
        if let Ok(path) = path_builder.build() {
            window.paint_path(path, self.color);
        }
    }
}

fn cursor_bounds(
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
