# GD Info

GD Info is a small native desktop utility for inspecting Geometry Dash players and levels using the Boomlings API.

## Features

- Player lookup by username with stats, IDs, icon IDs, colors, privacy, and social fields
- Level lookup by name or ID with rate data, coins, object count, version, copy state, original ID, and song metadata
- Structured native inspector UI instead of terminal-style result text
- Created levels shown on player profiles with paging, filtering, sorting, open, and copy-ID actions
- Level comments with in-section page navigation that refreshes comments without reloading the whole level
- Clickable cross-navigation for created levels, level creators, original levels, and commenter usernames
- Favorites for players and levels
- Recent search history with configurable limit
- Settings for theme, history limit, result font size, request timeout, and local-data clearing
- Local in-memory cache for repeated lookups during a session
- Granular copy actions for full results, IDs, names, and account/user IDs

## Tech Stack

- Rust
- egui / eframe
- reqwest
- serde

## Run

```bash
cargo run
```

## Build

```bash
cargo build --release
```

## Verify

```bash
cargo fmt && cargo test && cargo check
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

### macOS Gatekeeper

The macOS app is ad-hoc signed in CI, but it is not Apple-notarized because notarization requires an Apple Developer ID certificate.

If macOS says `"GD Info" is damaged and can't be opened`, remove the quarantine attribute after installing the app:

```bash
xattr -dr com.apple.quarantine "/Applications/GD Info.app"
```

Then open the app again.

To inspect release workflow runs:

```bash
gh run list
gh run view <run-id>
```

## Notes

This app uses the public Geometry Dash endpoints documented at <https://boomlings.dev/> for player, level, comment, and created-level data.
