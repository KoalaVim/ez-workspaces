## Context

See `proposal.md` — Why.

`browse()` in `src/browser/mod.rs` has two paths that open the session picker as the root view (`--repo` at ~line 183, auto-detect at ~line 190) and both collapse the picker's two non-accept outcomes into one:

```
SessionLoopResult::Accepted            => return Ok(())
SessionLoopResult::Cancelled | ViewAll => {}   // fall through to views::run
```

`session_action_loop` already distinguishes `Cancelled` from `ViewAll`, so the information exists — it is thrown away at the boundary. `browse_repo`, the wrapper the five view modules call, narrows the enum to `bool` (`Accepted` → `true`, everything else → `false`), which is why the `--repo` path cannot distinguish them either.

The session picker's keybind help is rendered by a *separate process*: the picker builds a preview command (`ez preview --session-actions --session-id {} …`, ~line 561) and fzf runs it. `preview_session_keybind_help()` in `src/browser/preview.rs` hardcodes `Esc → "Go back"`, so root-ness has to cross a process boundary to correct the label.

## Goals / Non-Goals

**Goals:**
- Preserve the exact behaviour of all five nested callers.
- Keep root-ness knowledge in `browse()`, which is the only place that knows how ez was launched.

**Non-Goals:**
- Changing how `ViewAll` behaves from a *nested* picker. Today `ctrl-a` in a picker reached from a view returns to that view rather than opening the global view (all five callers map `ViewAll` to `false`). That is a pre-existing quirk with its own fix; this change deliberately leaves it alone so the diff stays about cancellation.
- Distinguishing Escape from Ctrl-C. fzf reports both as exit 130 and `select_with_actions` maps both to `ActionResult::Cancel`; both should exit here, so nothing needs to change. (Splitting them via `--expect=esc` is a separate idea.)
- No new config key. `ctrl-a` already covers the "I actually wanted the global view" case.

## Decisions

### Widen `browse_repo` to return `SessionLoopResult` instead of `bool`

The root `--repo` caller needs all three outcomes; the nested callers need two. Returning the enum serves both and deletes the lossy narrowing rather than adding a second entry point.

Nested callers become `if matches!(result, SessionLoopResult::Accepted) { return … }` — mechanically equivalent to today's `if browse_repo(…)?`. Five call sites: `views/{owner,tree,label,workspace,repo}.rs`.

*Alternative considered:* keep `bool` and add `browse_repo_root()`. Rejected — two near-identical functions to keep in sync, and the `bool` would still be lying about what the picker returned.

### Both root paths handle the outcomes identically

```
Accepted  => return Ok(())
Cancelled => return Ok(())          // was: fall through
ViewAll   => {}                     // unchanged: fall through to views::run
```

`--repo` and auto-detect get the same treatment because both are the user naming an entry point explicitly. Splitting them would mean Escape behaves differently in two visually identical pickers.

Cancelling returns `Ok(())`, which exits 0 — the same status cancelling at the top level of any view already produces. This was verified during implementation and corrected: the specs originally claimed exit 130, which is wrong. `main.rs` only exits 130 for `EzError::Cancelled`, and the browser's view layer never produces it (`ActionResult::Cancel` → `Outcome::Done` → `Ok(())`).

Exiting 130 would also be a regression rather than a fix: the shell wrapper ends with `return $ret`, so a failure status would leave `$?` non-zero after every cancelled browse and light up the error indicator in most prompts. Cancellation is a normal outcome, so it exits successfully.

*Alternative considered:* `Cancelled` at root falls through only when the repo has exactly one session (nothing to pick). Rejected — behaviour that depends on invisible state is worse than a consistent rule.

### Thread root-ness to the preview as an explicit flag

Add an `is_root: bool` parameter to `session_action_loop` (and pass-through on `browse_repo`), used only to append a flag such as `--root` to the preview command it builds. `preview_session_keybind_help()` reads it and renders `Quit` instead of `Go back`.

The parameter carries no behavioural weight — the caller decides what cancellation means — so this is display plumbing only, and it stays a plain `bool` rather than a new enum.

*Alternative considered:* an env var (`EZ_PICKER_ROOT=1`) inherited by the preview subprocess. Rejected — a CLI flag is visible in the debug logs that already print the preview command, and `ez preview` is a documented surface.

*Alternative considered:* drop the label fix and let the help keep saying "Go back". Rejected — the help would actively mislead in the exact case this change is about.

## Risks / Trade-offs

- **A user has muscle-memory for Escape → global list.** → `ctrl-a` does it in one keypress and is listed in the picker's keybind help. Called out as a behavioural break in the proposal.
- **Adding a parameter to `session_action_loop` touches its call sites.** → It has two (`browse()` and `browse_repo`); the five view modules go through `browse_repo`.
- **`ez preview --root` becomes a public-ish surface.** → It only selects a help label; an unrecognised or absent flag falls back to the current `Go back` text.

## Migration Plan

None — no persisted state, config, or plugin contract changes. Rollback is reverting the commit.
