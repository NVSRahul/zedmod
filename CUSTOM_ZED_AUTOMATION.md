# Custom Zed Smooth Cursor Notes

This repository is a custom Zed fork with a native smooth cursor implementation integrated directly into Zed's Rust editor renderer.

## Important Summary

- No new crate was created.
- The feature was implemented by modifying existing Zed crates.
- The cursor work lives mainly in the `editor` crate.
- The feature is settings-driven through `editor.smooth_cursor`.
- GitHub Actions automation was added for macOS release builds and upstream sync.

## Goal

The goal of this fork is to provide a Neovim-like smooth cursor directly inside Zed, including optional smear / trail behavior, without relying on Zed's plugin system.

The reason for doing it this way is that Zed does not currently expose enough external rendering hooks to implement this as a plugin cleanly.

## Implementation Approach

The implementation uses Zed's native rendering path instead of creating a separate rendering layer.

### Core rendering approach

1. Keep the editor's real cursor position exact.
2. Store a separate animated visual cursor state for local cursors.
3. Smoothly move the visual cursor toward the real cursor target.
4. Optionally draw a smear trail between the animated position and target position.
5. Keep the animation state alive across frames using `request_animation_frame`.
6. Respect scrolling, cursor shape changes, and block cursor width changes.

### Design choices

- Native integration in Zed's existing editor rendering code
- No plugin implementation
- No new crate
- Settings-based control instead of hardcoded values only
- Local-cursor focused behavior
- Optional trail so the cursor can behave more like plain Neovim smoothing or more like Kitty smear

## Exact Files Changed

### Core smooth cursor feature

1. `crates/editor/src/editor.rs`
   Added persistent animation state storage on `Editor` for smooth cursor tracking.

2. `crates/editor/src/element.rs`
   Added the main smooth cursor implementation:
   - animation state stepping
   - smear trail generation
   - paint logic
   - coordinate-space fix for the ghost / offset trail issue
   - settings-driven smoothing and trail behavior

3. `crates/editor/src/editor_settings.rs`
   Added runtime `SmoothCursorSettings` and default values.

4. `crates/settings_content/src/editor.rs`
   Added schema support for `smooth_cursor` in settings content.

5. `crates/settings/src/vscode_import.rs`
   Wired the new setting through settings import handling.

6. `crates/settings_ui/src/page_data.rs`
   Added settings UI entries for the main cursor options.

7. `assets/settings/default.json`
   Added default `smooth_cursor` config values.

### Automation and maintenance

8. `.github/workflows/custom-zed-macos-aarch64.yml`
   Added GitHub Actions workflow for optimized Apple Silicon macOS builds.

9. `.github/workflows/sync-upstream.yml`
   Added scheduled/manual upstream sync workflow.

10. `script/install-downloaded-zed-mac`
    Added helper script to install a downloaded `.dmg`, `.app.tar.gz`, or `.app`.

11. `script/sync-upstream-local`
    Added local helper script for syncing upstream Zed changes into `smooth-cursor`.

12. `CUSTOM_ZED_AUTOMATION.md`
    This maintenance document.

## Settings Added

The custom setting is:

```json
{
  "smooth_cursor": {
    "enabled": true,
    "trail": false,
    "smooth_time": 45,
    "max_speed": 5200,
    "trail_opacity": 0.12,
    "trail_min_distance": 1.5
  }
}
```

### Meaning of settings

- `enabled`
  Turns smooth cursor movement on or off.

- `trail`
  Enables or disables the smear trail.

- `smooth_time`
  How quickly the animated cursor settles, in milliseconds.

- `max_speed`
  Maximum cursor movement speed in pixels per second.

- `trail_opacity`
  Opacity of the smear trail.

- `trail_min_distance`
  Minimum distance before the trail appears.

## Recommended Profiles

### Neovim-like smooth cursor

Use this if you want smooth cursor movement without a visible smear trail:

```json
{
  "smooth_cursor": {
    "enabled": true,
    "trail": false,
    "smooth_time": 45,
    "max_speed": 5200,
    "trail_opacity": 0.12,
    "trail_min_distance": 1.5
  }
}
```

### Kitty-like smear cursor

Use this if you want visible trail behavior:

```json
{
  "smooth_cursor": {
    "enabled": true,
    "trail": true,
    "smooth_time": 55,
    "max_speed": 4500,
    "trail_opacity": 0.16,
    "trail_min_distance": 1.5
  }
}
```

## How To Run Locally

### Dev run

Use an existing folder as the project path:

```sh
mkdir -p ~/Desktop/test-project-dir
cd /path/to/zed-main
cargo run ~/Desktop/test-project-dir
```

### Plain optimized binary run

```sh
cd /path/to/zed-main
cargo run --release ~/Desktop/test-project-dir
```

### Recommended optimized app bundle build

```sh
cd /path/to/zed-main
./script/bundle-mac aarch64-apple-darwin
```

Outputs:

- `target/aarch64-apple-darwin/release/Zed-aarch64.dmg`
- `target/aarch64-apple-darwin/release/dmg/Zed.app`

### Install the local bundled app

```sh
cd /path/to/zed-main
./script/bundle-mac -i aarch64-apple-darwin
```

## How To Test The Cursor

Test these behaviors:

1. Arrow-key movement for short cursor motion
2. Holding movement keys for continuous motion
3. Long jumps such as search results, page moves, or Vim motions
4. Scrolling while the cursor is still moving
5. Insert mode and normal mode if Vim mode is enabled
6. Block cursor shape, bar cursor shape, and underline behavior

## GitHub Actions Automation

### Build workflow

Workflow:

- `.github/workflows/custom-zed-macos-aarch64.yml`

What it does:

1. Builds a macOS Apple Silicon release bundle
2. Uploads:
   - `Zed-aarch64.dmg`
   - `Zed-aarch64.app.tar.gz`
3. Opts into Node 24 for JavaScript-based GitHub actions

### Sync workflow

Workflow:

- `.github/workflows/sync-upstream.yml`

What it does:

1. Runs on schedule and manual trigger
2. Fetches `zed-industries/zed` `main`
3. Attempts to merge it into `smooth-cursor`
4. Creates or updates an `auto/upstream-sync` PR if the merge is clean
5. Includes a summary of changed files in the PR body
6. Lets the macOS build workflow run on the PR

## Installing Downloaded Build Artifacts

Use:

```sh
cd /path/to/zed-main
script/install-downloaded-zed-mac ~/Downloads/Zed-aarch64.dmg
```

You can also install:

```sh
script/install-downloaded-zed-mac ~/Downloads/Zed-aarch64.app.tar.gz
```

or:

```sh
script/install-downloaded-zed-mac /path/to/Zed.app
```

If `/Applications` is not writable, run it again with `sudo`.

## Upstream Update Strategy

### Important truth

Future upstream sync cannot be made 100% automatic when Zed changes the same files as this custom cursor patch.

Automation can:

- fetch upstream
- attempt a merge
- open a PR when the merge is clean
- show a summary of changed files
- run the build automatically

Automation cannot safely decide how to resolve semantic conflicts in the same rendering code.

### Local update flow

```sh
cd /path/to/zed-main
script/sync-upstream-local
```

That does:

1. fetch `upstream/main`
2. merge it into `smooth-cursor`
3. remind you to rebuild

### Manual update flow

```sh
git fetch upstream
git checkout smooth-cursor
git merge upstream/main
cargo check -p editor
./script/bundle-mac aarch64-apple-darwin
```

### Recommended Git setting

Enable remembered conflict resolutions:

```sh
git config --global rerere.enabled true
```

This helps a lot if upstream touches nearby code repeatedly.

## What To Re-Read When Upstream Changes

Do not blindly reapply old edits by line number.

When upstream updates, re-read the changed files again before editing:

1. `crates/editor/src/element.rs`
2. `crates/editor/src/editor.rs`
3. `crates/editor/src/editor_settings.rs`
4. `crates/settings_content/src/editor.rs`
5. `crates/settings/src/vscode_import.rs`
6. `crates/settings_ui/src/page_data.rs`
7. `assets/settings/default.json`

These are the files most likely to matter for this custom cursor.

## Common Problems And Notes

### `target` folder becomes huge

Rust release builds for Zed can consume tens of gigabytes.

Clean it with:

```sh
cargo clean
```

If you want fewer heavy local builds, use GitHub Actions artifacts for release builds.

### `no credentials provided`

This comes from Zed edit prediction / AI features and does not affect the custom cursor.

### `Failed to open path in project`

This means you passed a file or a missing path instead of an existing directory to Zed.

Use a real folder:

```sh
mkdir -p ~/Desktop/test-project-dir
```

### `Failed to load user settings`

This means the user settings file had invalid JSON or invalid values.

Use simple valid JSON without comments if debugging settings problems.

### GitHub sync originally failed with `refusing to merge unrelated histories`

That was fixed by bootstrapping the custom branch onto real upstream Zed history once.

After that bootstrap, future `sync_upstream` runs should use normal Git merge history.

## Current Branch / Repo Expectations

Recommended branch layout:

- `main`
- `smooth-cursor`

Recommended default branch:

- `smooth-cursor`

Why:

- that branch contains the custom cursor work
- the sync workflow targets it
- the build workflow is intended to produce artifacts from it

## Final Maintenance Rule

If upstream changes a file you customized, read the new upstream file first, then adapt the patch carefully.

Do not assume old line numbers, old function boundaries, or old rendering behavior still match.
