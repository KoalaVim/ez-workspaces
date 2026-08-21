## Why

`ez plugin list` prints plugins using fixed-width `println!` formatting with hardcoded column widths. Plugin names or descriptions that exceed the padding break alignment, making the output hard to scan.

## What Changes

- Replace the manual `println!("{:<20} {:<19} {}", ...)` formatting in `list_plugins()` with a proper table that adapts column widths to content.
- Add a header row so column meaning is clear without memorization.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plugin-system`: The `list` subcommand output format changes from fixed-width columns to a dynamic-width table with headers.

## Impact

- `src/plugin/mod.rs`: `list_plugins()` function (lines 32-73).
- No API or config changes. Output-only change.
