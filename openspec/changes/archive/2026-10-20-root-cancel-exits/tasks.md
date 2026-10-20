## 1. Widen the session-picker return type

- [x] 1.1 `src/browser/mod.rs` — change `browse_repo`'s return type from `Result<bool>` to `Result<SessionLoopResult>`, returning the `session_action_loop` result directly instead of mapping `Accepted` → `true` / `Cancelled | ViewAll` → `false` (~line 244). Update the doc comment, which currently describes the `bool` contract.
- [x] 1.2 Update the five nested call sites to `matches!(…, SessionLoopResult::Accepted)`, preserving today's behaviour exactly (`ViewAll` still returns to the enclosing view): `src/browser/views/owner.rs:123`, `views/tree.rs:219`, `views/label.rs:160`, `views/workspace.rs:101`, `views/repo.rs:151`. Each needs `SessionLoopResult` added to its `use super::super::{…}` import.
- [x] 1.3 `cargo build` — confirm no remaining `if browse_repo(…)?` boolean uses.

## 2. Make cancellation exit at the root

- [x] 2.1 `src/browser/mod.rs` `browse()` — replace the `--repo` `if browse_repo(…)? { return Ok(()) }` block (~line 183) with a match: `Accepted | Cancelled => return Ok(())`, `ViewAll => {}` (fall through to `views::run`). Update the `// Esc/cancel: fall through to global view` comment, which becomes wrong.
- [x] 2.2 `src/browser/mod.rs` `browse()` — split the auto-detect match arm (~line 196) so `Accepted | Cancelled => return Ok(())` and `ViewAll => {}`.
- [x] 2.3 Confirm the exit code: cancelling a root picker returns `Ok(())` from `browse()`, exiting 0 — identical to cancelling at the top level of any view. The specs' original "code 130" claim was wrong (`main.rs` only exits 130 for `EzError::Cancelled`, which the view layer never produces) and would regress prompts via the wrapper's `return $ret`; delta spec and design.md corrected to "success status".

## 3. Correct the keybind help label

- [x] 3.1 `src/cli.rs` — add a hidden `#[arg(long, hide = true)] root: bool` flag to the `Preview` command (~line 132).
- [x] 3.2 `src/main.rs` — destructure `root` in the `Command::Preview` arm (~line 91) and pass it to `browser::preview`.
- [x] 3.3 `src/browser/preview.rs` — add the flag as a parameter to `preview` (line 16), thread it through `preview_session` to `preview_session_keybind_help`, and render `Esc → "Quit"` when set, keeping `"Go back"` otherwise (~line 675).
- [x] 3.4 `src/browser/mod.rs` — add an `is_root: bool` parameter to `session_action_loop` and a pass-through on `browse_repo`; append `--root` to the preview command it builds (~line 563) when set. Pass `true` from both root paths in `browse()`, `false` from the nested `browse_repo` callers.

## 4. Verification

Tasks 4.2-4.5 and 4.7 drive fzf interactively and must be run by hand in a real terminal — they cannot be automated from this session without taking over the live TTY.

- [x] 4.1 `cargo build && cargo test` — clean.
- [ ] 4.2 `cd` into a registered repo, run `ez`, press Escape — shell returns immediately, no global view, no second prompt. Repeat with Ctrl-C.
- [ ] 4.3 Run `ez --repo <path>` from an unrelated directory, press Escape — exits immediately.
- [ ] 4.4 Run `ez --all`, select a repo from the Workspace view, press Escape — returns to the Workspace view (unchanged). Repeat from the Repo and Tree views.
- [ ] 4.5 In a registered repo, run `ez`, press `ctrl-a` — global view opens; press Escape there — exits.
- [x] 4.6 Check the preview pane's keybind table shows `Esc  Quit` in a root picker and `Esc  Go back` in a picker reached from a view. Verified end-to-end by invoking the preview directly: `ez preview --session-actions --root --session-id <id> <repo>` prints `Esc  Quit`, and the same command without `--root` prints `Esc  Go back`.
- [ ] 4.7 `cd` into a *worktree* of a registered repo, run `ez`, press Escape — exits.
