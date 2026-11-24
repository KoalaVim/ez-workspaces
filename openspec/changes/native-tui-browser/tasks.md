## 1. Domain layer extraction (no behavior change)

- [x] 1.1 Create `src/browser/domain/` with `Row`/`RowKind` types and `Effect` enum; verify `make build` passes with zero warnings
- [x] 1.2 Move row building for Tree, Workspace, Repo, Owner, Label views into `domain/rows.rs` builder functions; rewire fzf views to use them; verify with unit tests on builders (tempdir repos) and `make test`
- [x] 1.3 Convert `preview.rs` renderers to write into `String` (`fmt::Write`) and move to `domain/preview.rs`; `ez preview` prints the returned string; verify `ez preview <repo>` output is byte-identical to before (golden test with colors forced)
- [x] 1.4 Split `session_action_loop` dispatch into `domain/actions.rs` handlers returning `Vec<Effect>`; add an fzf-side effect executor writing `cd_file`/`post_cmd_file`/relaunch marker; verify unit tests for each handler's effects and manual fzf smoke test (enter, new, delete, rename, plugin bind)
- [x] 1.5 Ensure all touched files are under 500 lines and run `make check`

## 2. Configuration

- [x] 2.1 Validate `selector.backend` (`fzf` | `native`) in `config set` and interactive setup; verify test rejects `skim` with an error listing accepted values
- [x] 2.2 Add `[native]` section (`preview_ratio` default 50, `snapshot` default true) to `EzConfig`; verify serde round-trip test and `ez config get native.preview_ratio`
- [x] 2.4 Honor `EZ_CONFIG_DIR` / `EZ_DATA_DIR` overrides in `src/paths.rs` (non-empty → replace base dir); verify unit tests for set/unset/empty and an integration test that `ez config set` with the override writes only under the override dir
- [x] 2.5 Add a `make dev` helper target documenting/launching a debug build against a dev dir (`EZ_CONFIG_DIR=$(DEV_DIR)/config EZ_DATA_DIR=$(DEV_DIR)/data`), defaulting `DEV_DIR` to `.dev/` in the repo (gitignored); verify `make dev ARGS="config get selector.backend"` reads the dev config
- [x] 2.3 Make `browse()` dispatch on backend and return a config error for unknown values; verify test for unknown backend error

## 3. TUI foundation

- [x] 3.1 Add `ratatui`, `crossterm`, `nucleo-matcher`, `ansi-to-tui` dependencies; verify `make build` with zero warnings
- [x] 3.2 Implement `tui/terminal.rs` guard (alt screen, raw mode, restore on Drop, panic hook, suspend/resume); verify manually that panic and Ctrl-C leave the terminal usable
- [x] 3.3 Implement `tui/keys.rs` fzf key-string parser with Alt/Alt-Shift normalization; verify unit tests for all default keybinds, uppercase-as-shift, named keys, and invalid strings
- [x] 3.4 Implement `tui/event.rs` sync loop (poll + mpsc results, dirty-flag redraw) and `tui/app.rs` skeleton; verify the app opens and Esc exits cleanly
- [x] 3.5 Implement `tui/list.rs` filtered list widget using nucleo-matcher on ANSI-stripped text with match highlighting; verify unit tests for ranking and ANSI-ignoring matches

## 4. Views and navigation

- [x] 4.1 Render Tree view from domain rows with preview pane (`preview_pane.rs`, ansi-to-tui, `preview_ratio`); verify manual run with `ez --select-by tree` and preview content matches fzf mode
- [x] 4.2 Add Workspace, Repo, Owner, Label views with view-switch keys preserving per-view query and cursor; verify manually the spec scenario "Switch and return preserves state"
- [x] 4.3 Implement `tui/nav.rs` navigation stack (view → repo session list → action menu), Esc pops, Esc at top exits, Ctrl-C exits with Interrupted; verify unit tests on the nav stack and manual check
- [x] 4.4 Implement session list with sort toggle and keybind help tables in preview; verify the custom-key-in-help scenario

## 5. Actions, effects and modals

- [x] 5.1 Implement `tui/effects.rs` executor (Cd, PostCmds+relaunch, Exec with suspend/resume, Refresh, Notify, Exit); verify with shell wrapper that enter-session cds and plugin bind relaunch works
- [x] 5.2 Implement `tui/modal.rs` `NativeSelector` (all 7 trait methods, nested loop on shared terminal, Ctrl-P/Esc/Ctrl-C semantics, modal confirm); verify unit-level tests where feasible and manual run of the name builder with back navigation
- [x] 5.3 Wire all session/repo actions (new, bare, from-dirty, delete, rename, cd, labels, notes open/cd, clone, remove, plugin binds) through domain handlers; verify manual parity checklist against fzf mode
- [x] 5.4 Refuse actions on rows that no longer exist in fresh data, with a notification; verify by deleting a session externally while the browser is open

## 6. Background loading and snapshot

- [x] 6.1 Implement `tui/workers.rs` fixed pool with generation ids; move branch, PR status, preview git log and bat notes to workers; verify rows and previews fill in and typing stays responsive (debug log timing)
- [x] 6.2 Run plugin `OnView`/view previews in workers; verify a plugin view loads without blocking input
- [x] 6.3 Implement `tui/snapshot.rs` (atomic TOML write on exit, versioned load on start, ignore corrupt/missing with debug log, honor `native.snapshot`); verify unit tests for round-trip, corrupt file, version mismatch
- [x] 6.4 Draw from snapshot first and swap in fresh rows keeping the cursor on the same value; verify the relaunch scenario shows rows on the first frame

## 7. Docs and finish

- [x] 7.1 Update README.md and `docs/user-guide.md` (enabling native backend, `[native]` options, differences from fzf); verify docs mention `selector.backend = "native"`
- [x] 7.2 Update `docs/design.md` diagrams (domain layer, two front ends, event loop/workers) and AGENTS.md source layout (`browser/domain/`, `browser/tui/`); verify by review
- [x] 7.3 Run `make check` (fmt, clippy, tests) with zero warnings and do a full manual parity pass in both backends
