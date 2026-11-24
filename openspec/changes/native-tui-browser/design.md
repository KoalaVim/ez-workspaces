## Context

See proposal.md for motivation. The current state that shapes the design:

- **Pull-style control flow.** Views (`src/browser/views/*.rs`) and the session action loop (`src/browser/mod.rs`, 1455 lines) build items, call `selector.select_with_actions(...)`, block until fzf exits, then dispatch on the returned key. About 31 call sites go through `InteractiveSelector`, including linear wizards in `session/name_builder.rs` and `config/mod.rs`.
- **Mixed concerns.** Row building (`read_dir`, `load_index`, `load_sessions`, `BranchCache`), formatting and fzf invocation are interleaved in each view. `preview.rs` writes straight to stdout with `println!` and `colored`.
- **Effects via files.** Accepting a session writes `cd_file`. Plugin binds write `post_cmd_file` and possibly `#EZ_RELAUNCH` (`browser/mod.rs:1007`). These commands must run in the user's parent shell, so **no in-process design can remove the relaunch**. It can only make it cheap.
- **Measured cost.** `ez preview <repo>` takes about 10–20 ms per call. That's acceptable alone, but it runs once per cursor move. The bigger cost is respawning fzf and rebuilding lists on every navigation step.
- **Conventions.** Synchronous only (no async runtime), files under 500 lines, `thiserror`, `log` debug logging, zero warnings, shell out to git, TOML for metadata.

## Goals / Non-Goals

**Goals:**
- One process and one terminal session for the whole native browse flow, with no visible teardown between levels.
- First paint in a few milliseconds on start and relaunch.
- Cursor movement and view switches never wait on git, disk or plugins.
- fzf mode keeps its current behavior and gets the shared domain layer for free.
- Wizard code (`name_builder`, config setup) is not rewritten.

**Non-Goals:**
- Removing `#EZ_RELAUNCH` or changing the shell wrapper or plugin protocol.
- Making native the default backend (a later decision, after use).
- Mouse support, themes or user-configurable native layouts beyond `preview_ratio`.
- Porting `fzf.extra_opts` semantics into native mode.

## Decisions

### D1. Event-loop app for browsing, trait impl for modals

```
                     browse()
                        │ selector.backend
          ┌─────────────┴──────────────┐
        "fzf"                       "native"
          │                            │
  browser/views/* (pull-style)   browser/tui::App (event loop)
  FzfSelector                    NativeSelector (modal, shares terminal)
          │                            │
          └────────────┬───────────────┘
                       ▼
           browser/domain/  (shared, pure-ish)
           rows.rs     build_*_rows() -> Vec<Row>
           preview.rs  render_*() -> String (ANSI)
           actions.rs  handle_*() -> Vec<Effect>
```

The native browser is an `App` with explicit state: a view set, a navigation stack, caches and a worker channel. `update(&mut App, Event)` returns effects, and `draw(&App, Frame)` renders. This is what makes view switching a re-render and lets data stream in.

Wizards stay pull-style. When an action needs one, such as new session, the handler calls `name_builder::run(&native_selector)` synchronously. `NativeSelector` holds a shared handle to the same `Terminal` (`Rc<RefCell<Terminal<_>>>`) and runs a **nested modal loop**: it draws the modal over a frozen snapshot of the app frame, reads keys until it has a result, then returns. The outer loop then redraws.

*Alternatives considered:*
- **RatatuiSelector only (option A).** Each trait call would run a blocking loop on a persistent terminal. It's cheaper, but views still rebuild on every call, state isn't kept across switches, and nothing can stream in. The user chose B.
- **Rewriting the wizards as app states.** That would make everything event-driven, but it duplicates logic used by both backends and adds risk for little speed gain, because wizards aren't hot paths.

### D2. Extract a shared domain layer first

Before any ratatui code, the view and action code is split into:
- `Row { display: String /*ANSI*/, value: String, kind: RowKind }` builders per view. `RowKind` carries what dispatch needs (repo path, session id, unmanaged worktree, root, plugin item).
- Preview renderers that write to a `String` (`fmt::Write`). The hidden `ez preview` subcommand becomes `print!("{}", render(...))`, so fzf mode keeps working unchanged.
- Action handlers that return `Vec<Effect>` instead of writing files or exiting directly:

```rust
enum Effect {
    Cd(PathBuf),                 // write cd_file, exit
    PostCmds { cmds: Vec<String>, relaunch: bool },
    Exec(Command),               // suspend TUI, run, resume
    Refresh(RefreshScope),       // reload rows/preview for scope
    Notify(String, Level),
    Exit,
}
```

Each front end has a small effect executor. Splitting `session_action_loop` is the riskiest step, so it lands with tests on the domain functions before the TUI exists. This also keeps every file under 500 lines.

*Alternative:* building the TUI directly on today's code. Rejected: it would require copying the dispatch logic, and the two backends would drift apart.

### D3. Synchronous event loop with worker threads

```
 main thread                               workers (std::thread)
 ┌──────────────────────────────┐          ┌──────────────────────┐
 │ loop {                       │  jobs    │ branch lookups       │
 │   if event::poll(16ms) {     │ ───────▶ │ PR status            │
 │     update(app, key)         │          │ git log for preview  │
 │   }                          │ ◀─────── │ bat note render      │
 │   while let Ok(r)=rx.try_recv│ results  │ plugin OnView        │
 │     apply(app, r)            │  (mpsc)  └──────────────────────┘
 │   if dirty { draw() }        │
 │ }                            │
 └──────────────────────────────┘
```

crossterm's `event::poll` with a short timeout plus an `mpsc` result channel covers this, so no async runtime is needed. Results carry a generation id, so stale results (for example, after a refresh) are dropped. `BranchCache` already uses a `Mutex`, so it's shared with workers through an `Arc`. Redraws happen only when state changed.

*Alternative:* tokio. Rejected because it breaks the no-async convention and isn't needed.

### D4. Preview: ANSI strings converted with `ansi-to-tui`

Preview renderers keep producing ANSI text through `colored`. The native pane converts the text with `ansi-to-tui` into a `Paragraph`. The output is identical in both backends, there's one renderer, and the keybind tables (which use ANSI-aware padding) keep working. Previews are cached per `(row value, data generation)`. Slow sections are rendered as placeholders first and spliced in when workers finish.

`colored` checks whether stdout is a TTY to decide on colors. The renderers force colors with `colored::control::set_override(true)` for the duration of a native render, unless `--no-color` is set.

*Alternative:* building ratatui `Text` natively. Rejected because it duplicates around 770 lines of rendering for both backends.

### D5. Fuzzy matching: `nucleo-matcher`

Use the low-level `nucleo-matcher` crate (synchronous, fzf-like scoring, returns match indices for highlighting) and match against ANSI-stripped display text. Lists are small (hundreds of rows), so matching runs on every keystroke on the main thread. The higher-level `nucleo` crate's threaded injector isn't needed.

*Alternatives:* `fuzzy-matcher` (the skim algorithm), which is slower and less actively maintained. Plain substring matching would be too big a UX regression from fzf.

### D6. Snapshot for instant first paint

On exit, the native browser writes `<data_dir>/ez/browser-snapshot.toml` with `{ version, view, rows per view (display, value, kind) }`. The file is written atomically (temp file plus rename). On start, it's loaded first, if `version` matches and `[native] snapshot` is true, and drawn immediately. Fresh rows are then built in the background and swapped in, keeping the cursor on the same `value`. TOML follows the project convention, and parsing a few hundred rows is well under a millisecond.

*Alternative:* bincode or JSON for speed. Not needed at this size, and TOML keeps the file inspectable.

### D7. Keybind translation

`browser/tui/keys.rs` parses fzf key syntax into crossterm `KeyEvent`, building a `HashMap<KeyEvent, Action>` at startup from `[keybinds]` and plugin binds. Rules: `ctrl-x`, `alt-x`, `alt-shift-x`, uppercase letters mean shift, and named keys (`enter`, `esc`, `tab`, `btab`, `space`, `bspace`, `up`/`down`/…, `f1`–`f12`). Terminals report Alt-Shift-letter in different ways (uppercase char with ALT, or lowercase with ALT|SHIFT), so lookups normalize both forms. Unknown strings produce a warning and the binding is skipped.

### D8. Terminal lifecycle and suspend

A `TerminalGuard` (RAII) enters the alternate screen and raw mode, and restores them on `Drop`. A panic hook restores the terminal before printing. `Effect::Exec` calls `guard.suspend()` (leave the alternate screen, disable raw mode), runs the command with inherited stdio, then `guard.resume()` and forces a full redraw.

### D9. Module layout (each file under 500 lines)

```
src/browser/domain/   rows.rs, preview.rs (moved from browser/preview.rs),
                      actions.rs, effect.rs
src/browser/tui/      mod.rs (run entry), app.rs, event.rs, nav.rs,
                      list.rs, preview_pane.rs, keys.rs, workers.rs,
                      snapshot.rs, modal.rs (NativeSelector), effects.rs,
                      terminal.rs (guard, suspend)
```

## Risks / Trade-offs

- **[Domain extraction breaks fzf mode]** → Do it as a separate first phase with unit tests on row builders and action handlers, and manually check fzf mode before starting the TUI.
- **[Nested modal loop and app state get out of sync]** (for example, the wizard deletes a session while the app holds stale rows) → Modals return control to the handler, and the handler always emits `Effect::Refresh` for the affected scope.
- **[Alt and Alt-Shift key reporting differs by terminal]** → Normalize as described in D7. Log every unmapped key event at debug level for troubleshooting.
- **[Snapshot shows data that has since been deleted]** → Rows are display-only until fresh data arrives. Actions on a row are checked against fresh data and refused with a notification if the target no longer exists.
- **[`colored` TTY detection]** → Force the color override during in-process renders (D4).
- **[Two front ends to maintain]** → fzf views are frozen apart from bug fixes, and all behavior lives in the domain layer. New features land in the domain layer first.
- **[Binary size and compile time from new dependencies]** → ratatui and crossterm are moderate. They're accepted for this feature.

## Migration Plan

1. Ship the domain extraction alone. Behavior doesn't change, fzf stays the default.
2. Ship the native browser behind `selector.backend = "native"` (opt-in), documented as experimental.
3. Rollback is `ez config set selector.backend fzf`. The snapshot file is a cache and can be deleted.
4. Switching the default to native is a separate future change.

## Open Questions

- Preview cache invalidation granularity (per row vs. per repo) can be tuned after profiling. It doesn't affect specs or tasks.
- Whether workers should be a fixed pool or a thread per job: start with a small fixed pool (for example, 4) and adjust after measuring.
