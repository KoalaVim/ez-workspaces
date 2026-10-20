## Why

When the session picker is the *entry point* — `ez` run inside a registered repo (auto-detect) or `ez --repo <path>` — pressing Escape or Ctrl-C does not exit. It falls through to the global browser view, so the user has to cancel twice to get back to their shell.

This contradicts the `interactive-browser` spec's own "Consistent back navigation" requirement ("Back navigation SHALL never skip levels or exit the browser unexpectedly"; "Escape at top level exits"). Everywhere else Escape *pops* a navigation level; here it *pushes* a view that was never popped from, moving the user deeper into the app instead of out of it.

The deliberate "take me to the global view" gesture already exists as the `view_all` keybind (default `ctrl-a`), so Escape is a redundant and worse second path to it.

## What Changes

- Cancelling the session picker when it is the root view exits ez (exit code 130) instead of falling through to the global browser view.
- Applies to both root entry paths:
  - auto-detect (bare `ez` inside a registered repo or one of its worktrees)
  - `ez --repo <path>`
- `SessionLoopResult::ViewAll` keeps its current behaviour: it still falls through to the global browser view. Only `Cancelled` changes.
- Nested entry is unaffected: selecting a repo from Workspace/Repo/Tree/Owner/Label views still returns to that view on Escape.
- The session picker's preview-pane keybind help shows `Esc → Quit` when the picker is the root view, and keeps `Esc → Go back` when it was reached from a view.

**BREAKING** (behavioural, for interactive use): users who relied on Escape-from-detected-repo landing in the global list must press `ctrl-a` instead. No config, API, or on-disk format changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `interactive-browser`: the "Consistent back navigation" requirement gains scenarios for the root-entry session picker (Escape/Ctrl-C exits, does not fall through); the "Auto-detect current repo" requirement gains the cancel-exits scenario; the "Session action loop" requirement's "Cancel returns to view layer" scenario is narrowed to nested entry only; and the Escape help label (root vs nested) is specified there rather than in the "Preview pane" requirement.

`session-view-all-bind` needs no delta — all of its scenarios remain true; `view_all` simply becomes the only path from the session picker to the global browser view.


## Impact

- `src/browser/mod.rs` — `browse()`: the `--repo` fall-through (`browse_repo` returning `false`) and the auto-detect match arm that currently treats `Cancelled` and `ViewAll` identically. `browse_repo`'s `bool` return no longer carries enough information for the root caller to distinguish the two outcomes.
- `src/browser/preview.rs` — `preview_session_keybind_help()` hardcodes `Esc → "Go back"`; needs to render `"Quit"` for root entry.
- `src/cli.rs` / preview invocation in `src/browser/mod.rs` — the preview subprocess command needs a flag carrying root-ness.
- No changes to config, plugins, session/repo stores, or the shell wrapper.
