# AGENTS.md

Guidance for AI coding agents working on GD Info.

## Project Overview

GD Info is a lightweight native Rust desktop utility for Geometry Dash lookups.

Keep the app simple:

- Single window
- No browser technologies
- No Electron, React, or Tauri
- Minimal dependencies
- Fast startup
- Low memory usage
- Utility-style UI, not a dashboard

## Architecture

Important source files:

- `src/main.rs` starts the eframe app.
- `src/ui.rs` owns egui state and layout.
- `src/api.rs` owns Boomlings API requests and parsing.
- `src/models.rs` owns app data structs.
- `src/storage.rs` owns local search history storage.
- `src/icon_renderer.rs` owns icon image fetching, caching, decoding, and egui texture conversion.

## API Rules

Use the Boomlings API for all Geometry Dash data:

- Players
- Levels
- Comments
- Created levels
- Stats
- IDs
- Colors
- Icon IDs

Only use GDBrowser IconKit for rendered icon image assets.

Do not use GDBrowser as a replacement for Boomlings data endpoints. GDBrowser is slower and not intended to replace direct Boomlings API access.

## Icon Rendering

Player icon data comes from Boomlings user objects:

- Cube ID
- Primary color
- Secondary color
- Glow enabled

`src/icon_renderer.rs` may call GDBrowser IconKit only to fetch icon image assets. It must cache generated icons under `cache/icons/` and must not fail player profile loading if icon rendering fails.

## Verification

Before finishing code changes, run:

```bash
cargo fmt && cargo check
```

## Release Workflow

GitHub Actions release automation lives at `.github/workflows/release.yml`.

The workflow runs only when a version tag matching `v*` is pushed. A normal push to `master` will not run it.

To create a release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow builds on:

- `macos-latest`, producing `GD-Info.dmg`
- `windows-latest`, producing `gd-info.exe`
- `ubuntu-latest`, producing `GD-Info.AppImage`

The macOS app is ad-hoc signed in CI with `codesign --sign -`, but it is not Apple-notarized. Notarization requires Apple Developer ID credentials. If users see a macOS `damaged and can't be opened` warning, they can remove quarantine after installing:

```bash
xattr -dr com.apple.quarantine "/Applications/GD Info.app"
```

The final job uploads all three files to the GitHub Release for the tag.

Use `gh run list` and `gh run view <run-id>` to inspect workflow runs.

## Git Ignore

Generated files that should stay untracked:

- `target/`
- `cache/`
