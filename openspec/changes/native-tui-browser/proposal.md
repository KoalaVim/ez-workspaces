## Why

The fzf-based browser puts a process boundary around every navigation step. View switches, Esc/back and action menus each kill fzf and start a new instance, which rebuilds the list from disk and makes the screen flicker. The preview pane forks `ez preview` on every cursor move, so it lags while scrolling. Plugin binds that relaunch ez (`#EZ_RELAUNCH`) restart the whole process cold, losing every cache. A native, in-process TUI built on ratatui removes these boundaries, so the browser feels instant.

## What Changes

- Add a **native TUI browser**: a full-screen, single-process ratatui application with an event loop. It holds every view in memory, so switching views and going back are re-renders, not process restarts.
- Render previews **in-process** with per-item caching. Slow sections (git log, `bat`-rendered notes, PR status) load on background threads and appear when ready.
- Add **instant first paint**: the native browser persists a snapshot of the last-rendered rows and draws it immediately on start or relaunch, then refreshes branch and PR data in the background.
- Add **in-process fuzzy filtering** for lists (no fzf dependency in native mode).
- Add a native `InteractiveSelector` implementation that runs **modal prompts** (name builder, input, confirm, back-aware stages) on the browser's terminal. Wizard code is unchanged.
- Extract a **shared domain layer** from today's views: row building, preview content as ANSI strings, and action effects. The fzf and native front ends both use it.
- Accept **fzf-style key strings** from the existing `[keybinds]` config in native mode, so user configs work unchanged.
- Add `native` as a value for the existing `selector.backend` config key. **fzf stays the default** and keeps working with its current behavior and `[fzf]` options.
- Not changing: the plugin protocol, the shell wrapper, or the `cd_file`/`post_cmd_file`/`#EZ_RELAUNCH` contract. Shell-affecting effects still have to exit to the parent shell.

## Capabilities

### New Capabilities
- `native-tui-browser`: The full-screen native browser: views, navigation, fuzzy filtering, in-process preview, background loading, snapshot-based first paint, suspend/resume for external programs, and effect handling (cd, post commands, relaunch).

### Modified Capabilities
- `selector-abstraction`: The single injection point now picks a backend from `selector.backend` (`fzf` or `native`). A native selector implementation is added for modal prompts.
- `configuration`: `selector.backend` accepts `native`. A `[native]` section is added for native-only options.

## Impact

- **Code**: new `src/browser/tui/` module tree. `src/browser/mod.rs`, `src/browser/preview.rs` and `src/browser/views/*` are refactored to share the domain layer (row builders, preview-to-string, `Effect`). `src/browser/selector.rs` gains a backend factory. `src/config/model.rs` gains native config.
- **Dependencies**: `ratatui`, `crossterm`, `nucleo` (fuzzy matching), `ansi-to-tui` (render existing ANSI preview output). All are synchronous-compatible, so the no-async convention holds.
- **Data**: a new snapshot file under `<data_dir>/ez/` (a cache that is safe to delete).
- **Platforms**: crossterm supports macOS, Linux and Windows, so native mode doesn't need fzf installed.
- **Docs**: README, `docs/user-guide.md`, `docs/design.md` diagrams and AGENTS.md (new `browser/tui/` module).
