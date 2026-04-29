import re

with open("crates/editor/src/element.rs", "r") as f:
    content = f.read()

# 1. Replace the physics block (from SMOOTH_CURSOR_SETTLE_DISTANCE_PX to struct SelectionLayout)
with open("kitty_physics.rs", "r") as f:
    physics_code = f.read()

content = re.sub(
    r"const SMOOTH_CURSOR_SETTLE_DISTANCE_PX: f32 = 0\.35;.*?#\[derive\(Debug\)\]\nstruct SelectionLayout \{",
    physics_code + "\n#[derive(Debug)]\nstruct SelectionLayout {",
    content,
    flags=re.DOTALL
)

# 2. Replace the inside of layout_cursors
new_layout_code = """
                    let target_origin = point(x, y);
                    let target_bounds = cursor_bounds(target_origin, block_width, line_height, selection.cursor_shape);

                    if selection.is_newest {
                        editor.pixel_position_of_newest_cursor = Some(point(
                            text_hitbox.origin.x + x + block_width / 2.,
                            text_hitbox.origin.y + y + line_height / 2.,
                        ));
                    }

                    if !in_range {
                        continue;
                    }

                    let mut block_text = None;
                    if selection.cursor_shape == CursorShape::Block {
                        let display_row = cursor_position.row();
                        let display_column = cursor_position.column();
                        if let Some(line) = layout.layout_for_row(display_row) {
                            if display_column < line.len() {
                                let c = line.text[display_column as usize..].chars().next().unwrap();
                                let mut run = line.runs[run_for_index(&line.runs, display_column as usize)].clone();
                                run.color = window.theme().colors().editor_background;
                                run.weight = FontWeight::BOLD;
                                let mut shaped_line = window.text_system().shape_line(
                                    c.to_string().into(),
                                    cx.text_style().font_size,
                                    &[run],
                                );
                                shaped_line.paint_color = Some(window.theme().colors().editor_background);
                                block_text = Some(shaped_line);
                            }
                        }
                    }

                    let mut smooth_trail = None;
                    let hide_local_cursor = selection.is_local && !show_local_cursors;
                    if smooth_cursor_enabled && selection.is_local {
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

content = re.sub(
    r"let target_origin = point\(x, y\);.*?let cursor_name = selection\.user_name\.clone\(\)\.map",
    new_layout_code.strip() + "\n                    let cursor_name = selection.user_name.clone().map",
    content,
    flags=re.DOTALL
)

with open("crates/editor/src/element.rs", "w") as f:
    f.write(content)
