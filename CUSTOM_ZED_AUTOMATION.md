# Custom Zed Smooth Cursor

This repo is a private Zed fork with one main custom feature: a native smooth
cursor with an optional Kitty-style smear trail.

The goal is simple:

- keep `main` as clean upstream Zed code
- keep all custom cursor work on `smooth-cursor`
- make future Zed updates easy to compare and maintain

## Branches

Use the branches like this:

- `main`
  Clean Zed upstream code. This should match `zed-industries/zed` `main`.

- `smooth-cursor`
  Your custom branch. This contains upstream Zed plus the smooth cursor changes,
  build workflows, install helper, and this maintenance note.

To compare your custom work later, compare:

```sh
main...smooth-cursor
```

On GitHub, open:

```text
https://github.com/NVSRahul/zedmod/compare/main...smooth-cursor
```

## What Was Changed

No new crate was created. The feature is kept inside the existing Zed editor
crate so the build stays close to upstream.

Runtime files:

- `crates/editor/src/smooth_cursor.rs`
  The smooth cursor engine. This owns the animation state, corner easing,
  trail shape, shader-inspired fade math, and painting.

- `crates/editor/src/editor.rs`
  Registers the module and stores smooth cursor state per editor selection.

- `crates/editor/src/element.rs`
  Connects Zed's normal cursor layout and paint flow to the smooth cursor
  engine.

- `crates/editor/src/editor_settings.rs`
  Runtime settings and default values for smooth cursor behavior.

- `crates/settings_content/src/editor.rs`
  User settings schema for `smooth_cursor`.

- `crates/settings/src/vscode_import.rs`
  Keeps settings import code compiling with the extra editor setting.

- `crates/settings_ui/src/page_data.rs`
  Adds smooth cursor settings to the settings UI.

- `assets/settings/default.json`
  Adds default `smooth_cursor` values.

Support files:

- `.github/workflows/custom-zed-macos-aarch64.yml`
  Builds the macOS Apple Silicon app and uploads artifacts.

- `.github/workflows/sync-upstream.yml`
  Tries to merge latest upstream Zed into `smooth-cursor`.

- `script/install-downloaded-zed-mac`
  Installs a downloaded `.dmg`, `.app.tar.gz`, or `.app`.

- `script/sync-upstream-local`
  Local helper for syncing upstream manually.

- `script/kitty_cursor_patch/README.md`
  Archive note only. It is not used by the build.

## Cursor Design

Zed's real cursor position is never delayed. Only the painted local cursor is
animated.

The animation works like this:

1. Zed computes the real cursor rectangle as usual.
2. The smooth cursor state keeps four animated corners for the painted cursor.
3. Each corner moves toward the real cursor using Kitty-style fast and slow
   exponential decay.
4. The leading edge moves faster than the trailing edge.
5. The smear trail is painted first.
6. The real cursor is painted on top, so editing behavior stays exact.

The newest trail fade is based on `cursor_smear_fade_final_blaze.glsl`.

The Rust port keeps the important parts:

- strong head, softer tail
- fade based on animation progress
- fade based on distance from the live cursor
- direction-aware hex-like smear shape
- layered halo, body, and core paint

It does not copy shader colors directly. Zed still uses the current theme cursor
color, then derives the trail colors from that.

## Settings

Example:

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

Fields:

- `enabled`
  Turns smooth cursor rendering on or off.

- `trail`
  Turns the smear trail on or off.

- `smooth_time`
  Slow trailing-edge smoothing time in milliseconds.

- `leading_smooth_time`
  Faster leading-edge smoothing time in milliseconds.

- `trail_opacity`
  Trail opacity multiplier from `0.0` to `1.0`.

- `trail_min_distance`
  Minimum movement distance before the trail appears.

Cleaner profile:

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

Smear profile:

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

## Build Locally

Dev run:

```sh
mkdir -p ~/Desktop/test-project-dir
cd /path/to/zed-main
cargo run ~/Desktop/test-project-dir
```

Optimized run:

```sh
cd /path/to/zed-main
cargo run --release ~/Desktop/test-project-dir
```

macOS bundle:

```sh
cd /path/to/zed-main
./script/bundle-mac aarch64-apple-darwin
```

Local outputs:

- `target/aarch64-apple-darwin/release/Zed-aarch64.dmg`
- `target/aarch64-apple-darwin/release/Zed-aarch64.app.tar.gz`

Install a downloaded build:

```sh
script/install-downloaded-zed-mac /path/to/Zed-aarch64.dmg
```

The installer also accepts:

```sh
script/install-downloaded-zed-mac /path/to/Zed-aarch64.app.tar.gz
script/install-downloaded-zed-mac /path/to/Zed.app
script/install-downloaded-zed-mac "/path/to/Zed Dev.app"
```

## GitHub Builds

The build workflow is:

```text
custom_zed_macos_aarch64
```

Artifacts are not in GitHub Releases. They are inside the workflow run.

To download:

1. Open the repo on GitHub.
2. Open `Actions`.
3. Open a successful `custom_zed_macos_aarch64` run.
4. Scroll to `Artifacts`.
5. Download `Zed-aarch64.dmg` or `Zed-aarch64.app.tar.gz`.

## Future Updates

Normal update flow:

```sh
git fetch upstream
git checkout smooth-cursor
git merge upstream/main
cargo check -p editor
cargo check -p settings_ui
git push origin smooth-cursor
```

After `smooth-cursor` is updated, keep `main` clean:

```sh
git push origin upstream/main:main
```

If GitHub rejects that because `main` moved differently, use a protected/manual
update on GitHub or push with lease only when you are sure `main` should be
exactly upstream Zed.

## Conflict Rule

When upstream touches one of the custom files, re-read the upstream version
first. Then re-apply only the small smooth cursor integration.

Most important files to check during conflicts:

- `crates/editor/src/smooth_cursor.rs`
- `crates/editor/src/editor.rs`
- `crates/editor/src/element.rs`
- `crates/editor/src/editor_settings.rs`
- `crates/settings_content/src/editor.rs`
- `crates/settings/src/vscode_import.rs`
- `crates/settings_ui/src/page_data.rs`
- `assets/settings/default.json`

Keep broad patch scripts out of the live workflow. The clean shape is one cursor
module plus small Zed integration hooks.

## Validation

Run:

```sh
cargo fmt -p editor -p settings -p settings_content -p settings_ui
cargo check -p editor
cargo check -p settings_ui
```

Manual cursor checks:

- short arrow-key moves
- held movement keys
- long jumps like `gg`, `G`, search, and page movement
- scrolling while the cursor is animating
- Vim normal and insert cursor shape changes
