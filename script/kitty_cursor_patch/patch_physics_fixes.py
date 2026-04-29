import re

with open("crates/editor/src/element.rs", "r") as f:
    content = f.read()

# 1. Update `retarget` to remove stale and distance snapping
old_retarget = """
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
"""

new_retarget = """
    fn retarget(
        &mut self,
        display_point: DisplayPoint,
        bounds: Bounds<Pixels>,
        shape: CursorShape,
        _now: Instant,
    ) {
        // We removed the stale and distance snapping checks here!
        // This ensures the cursor ALWAYS flies from its last known position
        // to the new position, even on `gg` (top of file) or `Shift+G` (bottom of file).
        
        self.target_display_point = display_point;
        self.target_bounds = bounds;
        self.shape = shape;
    }
"""

content = content.replace(old_retarget.strip(), new_retarget.strip())

# 2. Update `step` to fix the trail fading and decay scaling
old_step_trail = """
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
"""

new_step_trail = """
        let trail = if settings.trail && settings.trail_opacity > 0.0 && animating {
            let max_dist = (0..4)
                .map(|i| {
                    let d_x = target_corners[i].x.as_f32() - self.corners[i].x.as_f32();
                    let d_y = target_corners[i].y.as_f32() - self.corners[i].y.as_f32();
                    (d_x * d_x + d_y * d_y).sqrt()
                })
                .fold(0.0f32, f32::max);

            if max_dist > settings.trail_min_distance {
                // Ensure the tail stays fully visible during fast motion.
                // We only start fading the alpha when it's extremely close to stopping (within half a line height).
                let visibility_ratio = (max_dist / (self.target_bounds.size.height.as_f32() * 0.5)).clamp(0.0, 1.0);
                let trail_alpha = settings.trail_opacity.clamp(0.0, 1.0) * visibility_ratio;
                
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
"""

content = content.replace(old_step_trail.strip(), new_step_trail.strip())


# 3. Clean up the decay variables in step
old_decay = """
            // Kitty's default exponential ease parameters
            let decay_fast = 0.03;
            let decay_slow = settings.smooth_time.as_secs_f32().max(0.04);
"""

new_decay = """
            // Kitty's default exponential ease parameters.
            // Fast decay is the leading edge (snaps instantly).
            // Slow decay is the trailing edge (stretches elastically).
            let decay_fast = 0.03;
            // 300ms in settings = 0.3 seconds. We clamp it so it never goes to 0 and breaks math.
            let decay_slow = settings.smooth_time.as_secs_f32().clamp(0.04, 2.0);
"""

content = content.replace(old_decay.strip(), new_decay.strip())

with open("crates/editor/src/element.rs", "w") as f:
    f.write(content)
