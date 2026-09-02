## Why

On Windows, `std::fs::canonicalize` returns paths with the `\\?\` extended-length prefix (e.g., `\\?\C:\Users\Ofir\workspace\...`). This prefix prevents `collapse_tilde` from recognizing the home directory, so every path in the browser UI shows the raw absolute form instead of the `~/...` shorthand. The result is cluttered, hard-to-scan path displays.

## What Changes

- Update `paths::normalize` to strip the `\\?\` prefix on Windows after calling `canonicalize`, so all normalized paths are clean for both comparison and display.
- Replace scattered direct `path.canonicalize()` calls with `paths::normalize(path)` where the result may flow to display or be compared against registered repo paths, ensuring consistent prefix stripping across the codebase.

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

_(none — this is a platform bug fix; the existing path-display behavior is correct on Unix and the spec-level requirements are unchanged)_

## Impact

- **Code**: `src/paths.rs` (normalize function), plus ~18 direct `canonicalize()` call sites in `src/session/mod.rs`, `src/browser/mod.rs`, `src/repo/mod.rs`, `src/session/cursor.rs`, `src/session/current.rs`.
- **Behavior**: On Windows, browser paths display with `~` instead of `\\?\C:\Users\<user>\...`. On Unix, no change.
- **Dependencies**: None.
