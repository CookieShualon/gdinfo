# GD Info

GD Info is a small native desktop utility for looking up Geometry Dash players and levels using the Boomlings API.

## Features

- Player lookup by username
- Rendered player icon previews
- Level lookup by name or ID
- Created levels shown on player profiles
- Click a created level to load its details
- Level comments with comment page navigation
- Copy and clear results
- Last 10 searches saved locally
- Local icon image cache under `cache/icons/`

## Tech Stack

- Rust
- egui / eframe
- reqwest
- serde
- image

## Run

```bash
cargo run
```

## Build

```bash
cargo build --release
```

## Releases

Cross-platform releases are built with GitHub Actions from `.github/workflows/release.yml`.

The workflow runs when a version tag matching `v*` is pushed:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Release artifacts:

- `GD-Info.dmg` for macOS
- `gd-info.exe` for Windows
- `GD-Info.AppImage` for Linux

To inspect release workflow runs:

```bash
gh run list
gh run view <run-id>
```

## Notes

This app uses the public Geometry Dash endpoints documented at <https://boomlings.dev/> for player, level, comment, and created-level data.

GDBrowser IconKit is used only for icon image assets because Geometry Dash does not expose rendered icon images through the Boomlings endpoints. It is not used as a replacement API for Geometry Dash data.

Generated icon cache files are stored in `cache/icons/` and ignored by git.
