import re

with open("crates/editor/src/editor_settings.rs", "r") as f:
    content = f.read()

# Remove max_speed from SmoothCursorSettings struct
content = re.sub(r"\s*pub max_speed: f32,", "", content)
# Remove max_speed from SmoothCursorSettings default
content = re.sub(r"\s*max_speed: 4500\.0,", "", content)
# Remove max_speed from EditorSettings initialization
content = re.sub(r"\s*max_speed:\s*smooth_cursor\s*\.max_speed\s*\.unwrap_or\(smooth_cursor_defaults\.max_speed\),", "", content)

with open("crates/editor/src/editor_settings.rs", "w") as f:
    f.write(content)
