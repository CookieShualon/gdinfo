# Claude Notes

GD Info is a small native Rust desktop utility using egui/eframe.

## Hard Rules

- Keep Boomlings as the only source for Geometry Dash data.
- Do not replace player, level, comment, or created-level requests with GDBrowser requests.
- Keep the UI compact and utility-like.
- Avoid adding dependencies unless clearly needed.

## Current Modules

- `src/api.rs`: Boomlings API calls and response parsing.
- `src/ui.rs`: Single-window egui interface.
- `src/models.rs`: Shared structs.
- `src/storage.rs`: Last-search history.

## Test Command

Run this after edits:

```bash
cargo fmt && cargo check
```

## Releases

Release automation is defined in `.github/workflows/release.yml`.

It runs only for pushed tags matching `v*`, not for regular branch pushes.

Create a release by tagging the current commit and pushing the tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Artifacts created by the workflow:

- `GD-Info.dmg` from `macos-latest`
- `gd-info.exe` from `windows-latest`
- `GD-Info.AppImage` from `ubuntu-latest`

The macOS app is ad-hoc signed in CI but not Apple-notarized. Full notarization requires Apple Developer ID credentials. If macOS reports the app is damaged, remove quarantine after installing:

```bash
xattr -dr com.apple.quarantine "/Applications/GD Info.app"
```

Check release runs with:

```bash
gh run list
gh run view <run-id>
```
