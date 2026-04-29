const SMOOTH_CURSOR_SETTLE_DISTANCE_PX: f32 = 0.35;
const SMOOTH_CURSOR_SETTLE_VELOCITY_PX_PER_SEC: f32 = 14.0;
const SMOOTH_CURSOR_STALE_RESET: Duration = Duration::from_millis(180);
const SMOOTH_CURSOR_RESET_DISTANCE_PX: f32 = 1400.0;

#[derive(Clone, Debug)]
pub(crate) struct SmoothCursorAnimationState {
    target_display_point: DisplayPoint,
    current_origin: gpui::Point<Pixels>,
    target_origin: gpui::Point<Pixels>,
    origin_velocity: gpui::Point<f32>,
    current_block_width: Pixels,
    target_block_width: Pixels,
    block_width_velocity: f32,
    line_height: Pixels,
    shape: CursorShape,
    last_frame: Instant,
}

struct SmoothCursorFrame {
    origin: gpui::Point<Pixels>,
    block_width: Pixels,
    trail: Option<SmoothCursorTrail>,
    animating: bool,
}

#[derive(Clone, Debug)]
struct SmoothCursorTrail {
    points: SmallVec<[gpui::Point<Pixels>; 8]>,
    color: Hsla,
}

impl SmoothCursorAnimationState {
    fn new(
        display_point: DisplayPoint,
        origin: gpui::Point<Pixels>,
        block_width: Pixels,
        line_height: Pixels,
        shape: CursorShape,
        now: Instant,
    ) -> Self {
        Self {
            target_display_point: display_point,
            current_origin: origin,
            target_origin: origin,
            origin_velocity: point(0.0, 0.0),
            current_block_width: block_width,
            target_block_width: block_width,
            block_width_velocity: 0.0,
            line_height,
            shape,
            last_frame: now,
        }
    }

    fn snap_to(
        &mut self,
        display_point: DisplayPoint,
        origin: gpui::Point<Pixels>,
        block_width: Pixels,
        line_height: Pixels,
        shape: CursorShape,
        now: Instant,
    ) {
        self.target_display_point = display_point;
        self.current_origin = origin;
        self.target_origin = origin;
        self.origin_velocity = point(0.0, 0.0);
        self.current_block_width = block_width;
        self.target_block_width = block_width;
        self.block_width_velocity = 0.0;
        self.line_height = line_height;
        self.shape = shape;
        self.last_frame = now;
    }

    fn retarget(
        &mut self,
        display_point: DisplayPoint,
        origin: gpui::Point<Pixels>,
        block_width: Pixels,
        line_height: Pixels,
        shape: CursorShape,
        now: Instant,
    ) {
        let stale = now.duration_since(self.last_frame) > SMOOTH_CURSOR_STALE_RESET;
        let target_delta = point(
            origin.x - self.target_origin.x,
            origin.y - self.target_origin.y,
        );
        let reset_distance = ((target_delta.x.as_f32() * target_delta.x.as_f32())
            + (target_delta.y.as_f32() * target_delta.y.as_f32()))
        .sqrt();
        let line_height_changed = (self.line_height - line_height).as_f32().abs() > 0.5;

        if stale || line_height_changed || reset_distance > SMOOTH_CURSOR_RESET_DISTANCE_PX {
            self.snap_to(display_point, origin, block_width, line_height, shape, now);
            return;
        }

        if shape == self.shape && display_point == self.target_display_point {
            self.current_origin.x += target_delta.x;
            self.current_origin.y += target_delta.y;
            self.target_origin = origin;
            self.current_block_width = block_width;
            self.target_block_width = block_width;
            self.line_height = line_height;
            return;
        }

        self.target_display_point = display_point;
        self.target_origin = origin;
        self.target_block_width = block_width;
        self.line_height = line_height;
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
        let smooth_time = settings.smooth_time.as_secs_f32();
        let max_speed = settings.max_speed.max(1.0);

        self.current_origin.x = px(smooth_damp(
            self.current_origin.x.as_f32(),
            self.target_origin.x.as_f32(),
            &mut self.origin_velocity.x,
            smooth_time,
            max_speed,
            dt,
        ));
        self.current_origin.y = px(smooth_damp(
            self.current_origin.y.as_f32(),
            self.target_origin.y.as_f32(),
            &mut self.origin_velocity.y,
            smooth_time,
            max_speed,
            dt,
        ));
        self.current_block_width = px(smooth_damp(
            self.current_block_width.as_f32(),
            self.target_block_width.as_f32(),
            &mut self.block_width_velocity,
            smooth_time,
            max_speed,
            dt,
        ));

        let remaining = point(
            self.target_origin.x - self.current_origin.x,
            self.target_origin.y - self.current_origin.y,
        );
        let remaining_distance = ((remaining.x.as_f32() * remaining.x.as_f32())
            + (remaining.y.as_f32() * remaining.y.as_f32()))
        .sqrt();
        let velocity_magnitude = (self.origin_velocity.x * self.origin_velocity.x
            + self.origin_velocity.y * self.origin_velocity.y)
            .sqrt();
        let width_distance = (self.target_block_width - self.current_block_width)
            .as_f32()
            .abs();
        let animating = remaining_distance > SMOOTH_CURSOR_SETTLE_DISTANCE_PX
            || velocity_magnitude > SMOOTH_CURSOR_SETTLE_VELOCITY_PX_PER_SEC
            || width_distance > SMOOTH_CURSOR_SETTLE_DISTANCE_PX;

        if !animating {
            self.current_origin = self.target_origin;
            self.origin_velocity = point(0.0, 0.0);
            self.current_block_width = self.target_block_width;
            self.block_width_velocity = 0.0;
        }

        let trail = if settings.trail
            && settings.trail_opacity > 0.0
            && animating
            && remaining_distance > settings.trail_min_distance
        {
            let trail_alpha = settings.trail_opacity.clamp(0.0, 1.0)
                * (remaining_distance / self.line_height.as_f32().max(1.0)).min(1.0);
            SmoothCursorTrail::between(
                cursor_bounds(
                    self.current_origin,
                    self.current_block_width,
                    self.line_height,
                    self.shape,
                ),
                cursor_bounds(
                    self.target_origin,
                    self.target_block_width,
                    self.line_height,
                    self.shape,
                ),
                color.opacity(trail_alpha),
            )
        } else {
            None
        };

        SmoothCursorFrame {
            origin: self.current_origin,
            block_width: self.current_block_width,
            trail,
            animating,
        }
    }
}

impl SmoothCursorTrail {
    fn between(
        current_bounds: Bounds<Pixels>,
        target_bounds: Bounds<Pixels>,
        color: Hsla,
    ) -> Option<Self> {
        let hull = convex_hull(
            bounds_corners(current_bounds)
                .into_iter()
                .chain(bounds_corners(target_bounds)),
        );
        (hull.len() >= 3).then_some(Self {
            points: hull.into_iter().collect(),
            color,
        })
    }

    fn paint(&self, origin: gpui::Point<Pixels>, window: &mut Window) {
        let points = self
            .points
            .iter()
            .map(|point| *point + origin)
            .collect::<SmallVec<[gpui::Point<Pixels>; 8]>>();
        let mut path_builder = PathBuilder::fill();
        path_builder.add_polygon(&points, true);
        if let Ok(path) = path_builder.build() {
            window.paint_path(path, self.color);
        }
    }
}

fn smooth_damp(
    current: f32,
    target: f32,
    current_velocity: &mut f32,
    smooth_time: f32,
    max_speed: f32,
    delta_time: f32,
) -> f32 {
    let smooth_time = smooth_time.max(0.0001);
    let omega = 2.0 / smooth_time;
    let x = omega * delta_time;
    let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
    let change = (current - target).clamp(-max_speed * smooth_time, max_speed * smooth_time);
    let temp = (*current_velocity + omega * change) * delta_time;
    *current_velocity = (*current_velocity - omega * temp) * exp;
    let mut output = target + (change + temp) * exp;

    if (target - current > 0.0) == (output > target) {
        output = target;
        *current_velocity = 0.0;
    }

    output
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

fn convex_hull(points: impl IntoIterator<Item = gpui::Point<Pixels>>) -> Vec<gpui::Point<Pixels>> {
    let mut points = points.into_iter().collect::<Vec<_>>();
    points.sort_by(|a, b| {
        a.x.as_f32()
            .partial_cmp(&b.x.as_f32())
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                a.y.as_f32()
                    .partial_cmp(&b.y.as_f32())
                    .unwrap_or(Ordering::Equal)
            })
    });
    points.dedup_by(|a, b| {
        (a.x.as_f32() - b.x.as_f32()).abs() < 0.01 && (a.y.as_f32() - b.y.as_f32()).abs() < 0.01
    });
    if points.len() <= 2 {
        return points;
    }

    let mut lower = Vec::new();
    for point in &points {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], *point) <= 0.0
        {
            lower.pop();
        }
        lower.push(*point);
    }

    let mut upper = Vec::new();
    for point in points.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], *point) <= 0.0
        {
            upper.pop();
        }
        upper.push(*point);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn cross(a: gpui::Point<Pixels>, b: gpui::Point<Pixels>, c: gpui::Point<Pixels>) -> f32 {
    (b.x.as_f32() - a.x.as_f32()) * (c.y.as_f32() - a.y.as_f32())
        - (b.y.as_f32() - a.y.as_f32()) * (c.x.as_f32() - a.x.as_f32())
}

#[derive(Debug)]
struct SelectionLayout {
    id: usize,
    head: DisplayPoint,
    cursor_shape: CursorShape,
    is_newest: bool,
    is_local: bool,
    range: Range<DisplayPoint>,
    active_rows: Range<DisplayRow>,
    user_name: Option<SharedString>,
}

struct InlineBlameLayout {
    element: AnyElement,
    bounds: Bounds<Pixels>,
    buffer_id: BufferId,
    entry: BlameEntry,
}

impl SelectionLayout {
    fn new<T: ToPoint + ToDisplayPoint + Clone>(
