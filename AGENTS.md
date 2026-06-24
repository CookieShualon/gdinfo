# AGENTS.md

Guidance for AI coding agents working on GD Info.

## Project Overview

GD Info is a lightweight native Rust desktop utility for Geometry Dash lookups and inspection.

Keep the app simple:

- Single window
- No browser technologies
- No Electron, React, or Tauri
- Minimal dependencies
- Fast startup
- Low memory usage
- Utility-style inspector UI, not a dashboard

## Architecture

Important source files:

- `src/main.rs` starts the eframe app.
- `src/ui.rs` owns egui state and layout.
- `src/api.rs` owns Boomlings API requests and parsing.
- `src/models.rs` owns app data structs.
- `src/storage.rs` owns local app data storage: history, favorites, and settings.

## Current Product Surface

- Player lookup by username with stats, IDs, icon IDs, colors, privacy, and social fields.
- Player comment history is available only when the profile privacy says it is visible; the Show/Hide control belongs next to Comment history and page controls should refresh only comment history, not the whole profile.
- Level lookup by name or ID with rate data, coins, object count, version, copy state, decoded copy password when protected, original ID, two-player state, and song metadata.
- Structured egui inspector panels; do not regress to terminal-style result blobs as the primary UI.
- Created levels live inside player profiles and support paging, filtering, sorting, open, and copy-ID actions.
- Level comments live inside the level view. Comment page controls belong in the Comments section and should refresh only comments, not the whole level result.
- Favorites, recent searches, settings, and in-memory cache are app-local utility features.
- Cross-navigation should stay direct: created level to level, level creator to player, original ID to level, commenter to player.

## API Rules

Use the Boomlings API for all Geometry Dash data:

- Players
- Levels
- Comments
- Account comment history
- Created levels
- Stats
- IDs
- Colors
- Icon IDs
- Copy-state and copy-password metadata
- Song metadata

Do not use GDBrowser as a replacement for Boomlings data endpoints. GDBrowser is slower and not intended to replace direct Boomlings API access.

Level lookup primarily uses `getGJLevels21.php`. Use `downloadGJLevel22.php` narrowly for copy-state key `27`, because search responses can omit it. The key is encrypted with Geometry Dash's `26364` XOR scheme; decode it before labeling a level as free copy, not copyable, or password protected. Official song IDs from Boomlings are zero-based (`0` is Stereo Madness), and custom song records are separated with Boomlings song delimiters such as `~:~` and `~|~`.

Player comment history uses `getGJAccountComments20.php` and should remain separate from level comments. Do not reload the full player profile when switching account-comment pages.

## Verification

Before finishing code changes, run:

```bash
cargo fmt && cargo test && cargo check
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
