## Why

When ez auto-detects the current repo it jumps straight to that repo's session picker, skipping the global browser. There is no way to escape to the full browser from there — the user must quit and re-run `ez --all`. A keybind in the session view to switch to the global browser removes that round-trip.

## What Changes

- Add a configurable keybind (default: `ctrl-a`) in the session picker that breaks out to the global browser view (`ez --all` behavior).
- The bind exits the session action loop and falls through to `views::run`, the same path `--all` takes.

## Capabilities

### New Capabilities
- `session-view-all-bind`: Keybind in the session picker to switch to the global browser view.

### Modified Capabilities

## Impact

- `src/config/model.rs`: new `view_all` field in `KeybindsConfig`
- `src/browser/mod.rs`: handle the new key in `session_action_loop`, return a signal that `browse()` should fall through to the global view
