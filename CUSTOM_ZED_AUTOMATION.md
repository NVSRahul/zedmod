# Custom Zed Automation

This fork includes a lightweight GitHub Actions workflow for producing an optimized Apple Silicon macOS build of the custom Zed app with the smooth cursor changes.

## Local Release Build

For a fully optimized local macOS build:

```sh
cd /path/to/zed-main
./script/bundle-mac aarch64-apple-darwin
```

Output files:

- `target/aarch64-apple-darwin/release/Zed-aarch64.dmg`
- `target/aarch64-apple-darwin/release/dmg/Zed.app`

If you want to install the locally built app directly:

```sh
./script/bundle-mac -i aarch64-apple-darwin
```

## GitHub Actions Build

Workflow file:

- `.github/workflows/custom-zed-macos-aarch64.yml`

What it does:

1. Checks out the repo on a GitHub-hosted macOS runner
2. Uses the repo Rust toolchain
3. Runs `./script/bundle-mac aarch64-apple-darwin`
4. Uploads these artifacts:
   - `Zed-aarch64.dmg`
   - `Zed-aarch64.app.tar.gz`

To use it:

1. Put this code in a real GitHub repository or fork
2. Push your branch
3. Open the `Actions` tab
4. Run `custom_zed_macos_aarch64` manually, or let it run on pushes to `main` or `smooth-cursor`
5. Download the artifact you want

This workflow also runs on pull requests targeting `main` or `smooth-cursor`, so automated upstream sync PRs can build before you merge them.

## Installing a Downloaded Artifact

Install a downloaded artifact into `/Applications/Zed.app`:

```sh
cd /path/to/zed-main
script/install-downloaded-zed-mac ~/Downloads/Zed-aarch64.dmg
```

You can also install the raw app bundle tarball:

```sh
script/install-downloaded-zed-mac ~/Downloads/Zed-aarch64.app.tar.gz
```

If `/Applications` is not writable, re-run with `sudo`.

## Future Updates

The best long-term flow is:

1. Keep the custom cursor changes on a dedicated branch such as `smooth-cursor`
2. Add the original Zed repo as `upstream`
3. Pull upstream changes regularly
4. Rebase your branch
5. Push your branch and let GitHub Actions rebuild the binary

Important: this part cannot be made fully automatic if upstream changes the same editor files you customized. A workflow can fetch and attempt the merge, but conflict resolution still needs a human when Zed changes the same code paths.

What is automated here:

- `.github/workflows/sync-upstream.yml`
- `script/sync-upstream-local`

### Scheduled GitHub Sync

The GitHub workflow:

1. Runs every Monday and also supports manual trigger
2. Fetches `zed-industries/zed` `main`
3. Attempts to merge it into your `smooth-cursor` branch
4. Opens or updates a PR named `auto/upstream-sync`
5. Lets the macOS build workflow run on that PR

If the merge conflicts, the workflow fails before PR creation. That is your signal that upstream changed the same area and you need one manual merge pass.

### Local Sync

Inside a real Git clone, you can do the same locally:

```sh
cd /path/to/zed-main
script/sync-upstream-local
```

That will:

1. Fetch `upstream/main`
2. Merge it into `smooth-cursor`
3. Tell you to rebuild/check

Typical update flow in a real Git clone:

```sh
git remote add upstream https://github.com/zed-industries/zed.git
git fetch upstream
git checkout smooth-cursor
git rebase upstream/main
git push --force-with-lease origin smooth-cursor
```

If you prefer merge-based history for easier automation, this also works:

```sh
git fetch upstream
git checkout smooth-cursor
git merge upstream/main
git push origin smooth-cursor
```

For repeated manual conflict resolution, turn on Git’s remembered resolutions once:

```sh
git config --global rerere.enabled true
```

## Disk Usage

Large local Rust builds can consume tens of gigabytes.

To reclaim disk space:

```sh
cargo clean
```

If you prefer to keep local builds minimal, use GitHub Actions for release binaries and reserve local builds for quick iteration or debugging.
