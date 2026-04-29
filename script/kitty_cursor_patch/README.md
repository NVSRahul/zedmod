This directory is archival reference only.

The live smooth-cursor implementation is in:

- `crates/editor/src/smooth_cursor.rs`
- `crates/editor/src/element.rs`
- `crates/editor/src/editor_settings.rs`

These files are not used by the build anymore because the exploratory patch
scripts and duplicate physics snapshots made future rebases harder than direct
Rust maintenance.

Kitty source references used for the port:

- `kitty/cursor_trail.c`
- `kitty/shaders.c`
- `kitty/options/definition.py`

Future updates should modify the Rust implementation directly instead of
re-running patch scripts.
