## Context

When ez auto-detects the current repo (or `--repo` is passed), `browse()` calls `session_action_loop()` and returns immediately. There is no path back to the global browser — the user must quit and re-run `ez --all`. The session action loop currently returns `Result<bool>` — `true` for accepted, `false` for cancelled.

## Goals / Non-Goals

**Goals:**
- Add a keybind in the session picker to switch to the global browser
- Keep the change minimal — reuse the existing `views::run` path

**Non-Goals:**
- Changing how `--all` works on the CLI
- Adding navigation back from global browser to a specific repo's session picker

## Decisions

### Return type: enum instead of bool
`session_action_loop` returns `Result<bool>`. Add a `SessionLoopResult` enum with `Accepted`, `Cancelled`, and `ViewAll` variants. In `browse()`, when auto-detect or `--repo` gets `ViewAll`, fall through to `views::run` instead of returning.

**Alternative**: Use a sentinel error or a mutable flag. Both are less explicit and harder to extend.

### Default keybind: `ctrl-a`
`ctrl-a` is mnemonic for "all" and not currently used in the session picker. It's already used in the repo/directory views for clone (`alt-a`), but `ctrl-a` is free.

## Risks / Trade-offs

- `ctrl-a` conflicts with tmux prefix in default tmux configs. However, fzf already consumes the key before tmux sees it, so this is not a real conflict when the picker is active.
- The return-type change from `bool` to enum touches `browse_repo` as well (the `--repo` path). Both callers need updating — but there are only two.
