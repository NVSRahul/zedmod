# Custom Zed Smooth Cursor Notes

This repository is a custom Zed fork with a native smooth cursor integrated
directly into Zed's Rust editor renderer.

## Important Summary

- No new crate was created.
- The smooth cursor now lives in one dedicated internal module:
  `crates/editor/src/smooth_cursor.rs`
- The rest of the editor changes are thin integration hooks.
- Kitty-inspired reference material is archival only and is not part of the
  runtime build.

## Goal

Provide a Neovim-like smooth cursor inside Zed with optional Kitty-style smear
behavior, without depending on plugin hooks that Zed does not currently expose.

## Active Runtime Files

These are the only files you should expect to touch for the feature itself:

1. `crates/editor/src/smooth_cursor.rs`
   - Main cursor animation engine
   - Kitty-style 4-corner exponential decay math
   - Trail shape generation and painting
   - Shared cursor bounds helper

2. `crates/editor/src/editor.rs`
   - Registers the `smooth_cursor` module
   - Stores per-selection smooth cursor animation state on `Editor`

3. `crates/editor/src/element.rs`
   - Small integration layer between Zed's editor paint flow and the smooth
     cursor module
   - Cursor layout still lives here, but the animation engine does not

4. `crates/editor/src/editor_settings.rs`
   - Runtime `SmoothCursorSettings`
   - Default timing and trail values

5. `crates/settings_content/src/editor.rs`
   - JSON schema for `smooth_cursor`

6. `crates/settings/src/vscode_import.rs`
   - Keeps the new setting wired through settings import handling

7. `crates/settings_ui/src/page_data.rs`
   - Settings UI entries for smooth cursor options

8. `assets/settings/default.json`
   - Default user-visible `smooth_cursor` values

## Runtime Design

The implementation keeps Zed's real cursor position exact and only animates the
painted local cursor.

High-level flow:

1. Compute the real cursor target bounds in Zed's existing layout pass.
2. Store one smooth-cursor animation state per local selection ID.
3. Retarget the animation state every frame.
4. Move the four cursor corners independently using Kitty-style fast/slow
   exponential decay.
5. Paint the smear trail first, then paint the real cursor quad on top.

Why this structure is cleaner:

- The animation math is isolated in one module instead of being embedded into
  `element.rs`.
- Future upstream rebases mostly touch one new file and a few call sites.
- The settings layer now matches the actual runtime model.

## Settings Surface

The smooth cursor is configured through `smooth_cursor` in user settings:

```json
{
  "smooth_cursor": {
    "enabled": true,
    "trail": true,
    "smooth_time": 55,
    "leading_smooth_time": 30,
    "trail_opacity": 0.16,
    "trail_min_distance": 1.5
  }
}
```

Meaning:

- `enabled`
  Turns smooth cursor rendering on or off.

- `trail`
  Enables or disables the smear trail.

- `smooth_time`
  Slow decay time in milliseconds for the trailing edge.

- `leading_smooth_time`
  Fast decay time in milliseconds for the leading edge.

- `trail_opacity`
  Trail alpha multiplier from `0.0` to `1.0`.

- `trail_min_distance`
  Minimum cursor travel distance before painting the smear trail.

## Recommended Profiles

### Cleaner Neovim-like motion

```json
{
  "smooth_cursor": {
    "enabled": true,
    "trail": false,
    "smooth_time": 45,
    "leading_smooth_time": 25,
    "trail_opacity": 0.12,
    "trail_min_distance": 1.5
  }
}
```

### Kitty-like smear

```json
{
  "smooth_cursor": {
    "enabled": true,
    "trail": true,
    "smooth_time": 55,
    "leading_smooth_time": 30,
    "trail_opacity": 0.16,
    "trail_min_distance": 1.5
  }
}
```

## Local Development

### Dev run

```sh
mkdir -p ~/Desktop/test-project-dir
cd /path/to/zed-main
cargo run ~/Desktop/test-project-dir
```

### Optimized run

```sh
cd /path/to/zed-main
cargo run --release ~/Desktop/test-project-dir
```

### Bundle a macOS app

```sh
cd /path/to/zed-main
./script/bundle-mac aarch64-apple-darwin
```

Outputs:

- `target/aarch64-apple-darwin/release/Zed-aarch64.dmg`
- `target/aarch64-apple-darwin/release/Zed-aarch64.app.tar.gz`

### Install a downloaded or locally produced build

```sh
script/install-downloaded-zed-mac /path/to/Zed-aarch64.dmg
```

Also supports:

```sh
script/install-downloaded-zed-mac /path/to/Zed-aarch64.app.tar.gz
script/install-downloaded-zed-mac /path/to/Zed.app
```

## GitHub Actions

### Build workflow

File:

- `.github/workflows/custom-zed-macos-aarch64.yml`

It:

1. Builds the Apple Silicon macOS bundle
2. Packs the `.app` dynamically so `Zed.app` and `Zed Dev.app` both work
3. Uploads:
   - `Zed-aarch64.dmg`
   - `Zed-aarch64.app.tar.gz`

### Where to download artifacts

Artifacts are in the **Actions run**, not in GitHub Releases.

Open:

1. GitHub repo
2. `Actions`
3. The successful `custom_zed_macos_aarch64` run
4. `Artifacts`

Download either:

- `Zed-aarch64.dmg`
- `Zed-aarch64.app.tar.gz`

### Upstream sync workflow

File:

- `.github/workflows/sync-upstream.yml`

It:

1. Fetches `zed-industries/zed` `main`
2. Attempts to merge it into `smooth-cursor`
3. Creates or updates `auto/upstream-sync` if the merge is clean

## Kitty Reference Files

The folder below is archival only:

- `script/kitty_cursor_patch/README.md`

Do not treat it as live automation. It only points to the original Kitty source
files that informed the port.

## Future Update Workflow

When upstream Zed changes:

1. Sync upstream into `smooth-cursor`
2. Re-read these files before editing:
   - `crates/editor/src/smooth_cursor.rs`
   - `crates/editor/src/editor.rs`
   - `crates/editor/src/element.rs`
   - `crates/editor/src/editor_settings.rs`
   - `crates/settings_content/src/editor.rs`
   - `crates/settings_ui/src/page_data.rs`
   - `assets/settings/default.json`
3. Adapt the integration, not the entire renderer
4. Run validation

Useful commands:

```sh
git config --global rerere.enabled true
git fetch upstream
git checkout smooth-cursor
git rebase upstream/main
cargo check -p editor
cargo check -p settings_ui
```

## Validation

Recommended checks after cursor changes:

```sh
cargo check -p editor
cargo check -p settings_ui
```

Cursor behavior to test:

1. Short arrow-key moves
2. Held movement keys
3. Long jumps like `gg`, `G`, search navigation, page moves
4. Scrolling while the cursor is still animating
5. Vim normal/insert mode shape changes

## Maintenance Rule

If upstream changes a customized file, read the new upstream version first and
then adapt the small integration surface. Do not re-introduce broad patch
scripts or large inlined cursor logic into `element.rs`.
