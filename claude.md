# Claude Notes

GD Info is a small native Rust desktop utility using egui/eframe.

## Hard Rules

- Keep Boomlings as the only source for Geometry Dash data.
- Use GDBrowser IconKit only for rendered icon image assets.
- Do not replace player, level, comment, or created-level requests with GDBrowser requests.
- Keep the UI compact and utility-like.
- Avoid adding dependencies unless clearly needed.

## Current Modules

- `src/api.rs`: Boomlings API calls and response parsing.
- `src/icon_renderer.rs`: IconKit image fetch, local cache, PNG decode, egui texture helper.
- `src/ui.rs`: Single-window egui interface.
- `src/models.rs`: Shared structs.
- `src/storage.rs`: Last-search history.

## Test Command

Run this after edits:

```bash
cargo fmt && cargo check
```
