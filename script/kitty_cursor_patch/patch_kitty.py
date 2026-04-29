import re

with open("crates/editor/src/element.rs", "r") as f:
    content = f.read()

# 1. Replace the physics types
with open("kitty_physics.rs", "r") as f:
    physics_code = f.read()

content = re.sub(
    r"const SMOOTH_CURSOR_SETTLE_DISTANCE_PX: f32 = 0\.35;.*?#\[derive\(Debug\)\]\nstruct SelectionLayout \{",
    physics_code + "\n#[derive(Debug)]\nstruct SelectionLayout {",
    content,
    flags=re.DOTALL
)

# 2. Update layout_cursors calls
old_call = """
                    let mut smooth_trail = None;
                    let mut origin = target_origin;
                    let hide_local_cursor = selection.is_local && !show_local_cursors;
                    if smooth_cursor_enabled && selection.is_local {
                        let state = editor
                            .smooth_cursor_animations
                            .entry(selection.id)
                            .or_insert_with(|| {
                                SmoothCursorAnimationState::new(
                                    cursor_position,
                                    target_origin,
                                    block_width,
                                    line_height,
                                    selection.cursor_shape,
                                    now,
                                )
                            });
                        if hide_local_cursor {
                            state.snap_to(
                                cursor_position,
                                target_origin,
                                block_width,
                                line_height,
                                selection.cursor_shape,
                                now,
                            );
                        } else {
                            state.retarget(
                                cursor_position,
                                target_origin,
                                block_width,
                                line_height,
                                selection.cursor_shape,
                                now,
                            );
                            let smooth_frame =
                                state.step(now, player_color.cursor, &smooth_cursor_settings);
                            any_animated_cursor |= smooth_frame.animating;
                            origin = smooth_frame.origin;
                            smooth_trail = smooth_frame.trail;
                            block_width = smooth_frame.block_width;
                            if smooth_frame.animating
                                && selection.cursor_shape == CursorShape::Block
                            {
                                block_text = None;
                            }
                        }
                    }

                    if hide_local_cursor {
                        continue;
                    }

                    let mut cursor = CursorLayout {
                        color: player_color.cursor,
                        block_width,
                        origin,
                        line_height,
                        shape: selection.cursor_shape,
                        block_text,
                        cursor_name: None,
                        smooth_trail,
                    };
"""

new_call = """
                    let mut smooth_trail = None;
                    let hide_local_cursor = selection.is_local && !show_local_cursors;
                    if smooth_cursor_enabled && selection.is_local {
                        let target_bounds = cursor_bounds(target_origin, block_width, line_height, selection.cursor_shape);
                        let state = editor
                            .smooth_cursor_animations
                            .entry(selection.id)
                            .or_insert_with(|| {
                                SmoothCursorAnimationState::new(
                                    cursor_position,
                                    target_bounds,
                                    selection.cursor_shape,
                                    now,
                                )
                            });
                        if hide_local_cursor {
                            state.snap_to(
                                cursor_position,
                                target_bounds,
                                selection.cursor_shape,
                                now,
                            );
                        } else {
                            state.retarget(
                                cursor_position,
                                target_bounds,
                                selection.cursor_shape,
                                now,
                            );
                            let smooth_frame =
                                state.step(now, player_color.cursor, &smooth_cursor_settings);
                            any_animated_cursor |= smooth_frame.animating;
                            smooth_trail = smooth_frame.trail;
                            if smooth_frame.animating
                                && selection.cursor_shape == CursorShape::Block
                            {
                                block_text = None;
                            }
                        }
                    }

                    if hide_local_cursor {
                        continue;
                    }

                    let mut cursor = CursorLayout {
                        color: player_color.cursor,
                        block_width,
                        origin: target_origin,
                        line_height,
                        shape: selection.cursor_shape,
                        block_text,
                        cursor_name: None,
                        smooth_trail,
                    };
"""

content = content.replace(old_call.strip(), new_call.strip())

with open("crates/editor/src/element.rs", "w") as f:
    f.write(content)
