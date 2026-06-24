# Claude Notes

GD Info is a small native Rust desktop inspector using egui/eframe.

## Hard Rules

- Keep Boomlings as the only source for Geometry Dash data.
- Do not replace player, level, comment, or created-level requests with GDBrowser requests.
- Keep the UI compact, structured, and utility-like.
- Do not regress the primary results UI to terminal-style text blobs.
- Keep comment page controls inside the Comments section; changing comment pages should refresh only comments, not the whole level result.
- Avoid adding dependencies unless clearly needed.

## Current Product Surface

- Player lookup includes stats, IDs, icon IDs, colors, privacy, and social fields.
- Level lookup includes rate data, coins, object count, version, copy state, decoded copy password when protected, original ID, two-player state, and song metadata.
- Player profiles include created levels with paging, filtering, sorting, open, and copy-ID actions.
- Level views include comments with isolated comment pagination.
- App-local features include favorites, recent searches, settings, and in-memory lookup cache.
- Cross-navigation links created levels to levels, level creators to players, original IDs to levels, and commenters to players.

## Current Modules

- `src/api.rs`: Boomlings API calls and response parsing.
- `src/ui.rs`: Single-window egui interface.
- `src/models.rs`: Shared structs.
- `src/storage.rs`: Local app data persistence for history, favorites, and settings.

## Boomlings Parsing Notes

- Level lookup primarily uses `getGJLevels21.php`.
- Use `downloadGJLevel22.php` only to read copy-state key `27`, because search responses can omit it.
- Copy-state key `27` is encrypted with Geometry Dash's `26364` XOR scheme; decode it before labeling levels or showing protected copy passwords.
- Official Boomlings song IDs are zero-based: `0` is Stereo Madness.
- Custom song records use Boomlings song delimiters such as `~:~` and `~|~`.

## Test Command

Run this after edits:

```bash
cargo fmt && cargo test && cargo check
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
