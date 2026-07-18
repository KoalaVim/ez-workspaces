## Why

The core session and browser workflows have accumulated several UX rough edges and bugs since the initial implementation. Tree view sessions can't be opened properly, sessions created via CLI land as orphan roots instead of under `main`, back-navigation is broken after repo selection, and tmux session deletion often fails. Additionally, the session name builder is rigid — it only supports the staged prefix/ticket/name flow, but common workflows like creating sessions from GitHub PR links or Jira URLs aren't supported, and there's no "just type a name" shortcut. The browser also lacks visual polish (no tree glyphs in the session picker) and a dedicated cd keybind.

## What Changes

### Bug Fixes
- **Tree view session enter**: Selecting a session in tree view currently calls `write_cd_target` directly, bypassing the `accept_session` flow (which handles `on_enter` actions like tmux attach). Fix to use `accept_session` with `post_cmd_file` passthrough.
- **New session default parent**: `ez session new` and `ez session register` create sessions with `parent_id: None`, making them root-level siblings of `main`. They should default to being children of the default (main) session when no `--parent` is specified.
- **Back navigation after repo selection**: Escape in the session action loop should return to the view layer (previous view), not exit entirely. Audit all navigation levels to ensure Escape consistently goes back.
- **Tmux session kill reliability**: The detached reap worker for `OnSessionDelete` often fails to kill the tmux session. Investigate and fix the timing/process group issues.

### Features
- **README refinement**: Update the README to better reflect the current feature set and improve onboarding clarity.
- **Interactive builder flag**: Add a `--interactive` / `-i` flag to `ez session new` that forces the multi-stage name builder even when a name is provided on the CLI (currently the builder is only used when name is omitted).
- **Name builder mode selection**: Before starting the staged builder, present a mode picker:
  - **Full name** — skip stages, just type the whole name
  - **Build from parts** — current staged builder (prefix → ticket → name)
  - **From GitHub PR** — paste a PR URL, extract `pr<number>`, optionally invoke a plugin to fetch the branch name and set the worktree on it
  - **From Jira URL** — paste a Jira URL, extract the project key and ticket number (e.g. `PROJ-123`), then continue with the configured builder for the descriptive suffix
  - The available modes should be configurable.
- **Cd keybind in session picker**: Add a dedicated keybind (e.g. `alt-c`) in the session action loop that always cd's into the session, regardless of the configured `on_enter` action.
- **Enhanced branch fetch on session create**: When creating a session, also fetch the branch name (not just main/master) to check if it exists remotely and refresh it if stale locally.
- **Return to ez after tmux detach**: After detaching from a tmux session (`Ctrl-b d`), automatically return to the ez browser so the user can pick another session.
- **Tree glyphs in session picker**: Add box-drawing tree connectors (`├──`, `└──`, `│`) to the session picker (the fzf list shown after selecting a repo), matching the style used in the Tree view.

## Capabilities

### New Capabilities
- `name-builder-modes`: Configurable session name builder modes (full name, from parts, from GitHub PR, from Jira URL) with a mode selection step before the staged builder.

### Modified Capabilities
- `session-management`: Default new sessions under `main`, interactive builder flag, enhanced branch fetch
- `interactive-browser`: Fix tree view session enter, back navigation, cd keybind, tree glyphs in session picker, return-to-ez after tmux detach
- `plugin-system`: GitHub PR plugin hook for branch resolution
- `shell-integration`: Return-to-ez loop after tmux detach
- `configuration`: Name builder mode configuration, cd keybind configuration

## Impact

- **session/mod.rs**: `new_session`, `register_existing_worktree`, `create_child_session` — default `parent_id` logic
- **session/name_builder.rs**: Mode selection step, new mode handlers (PR, Jira, full name)
- **browser/views/tree.rs**: Use `accept_session` instead of `write_cd_target`, pass `post_cmd_file`
- **browser/mod.rs**: Session action loop back-navigation, cd keybind, tree glyph rendering
- **browser/views/*.rs**: Audit Escape handling for consistent back navigation
- **plugin/**: Git-worktree plugin fetch enhancement, potential new PR-resolution plugin hook
- **config/model.rs**: New keybind for cd, name builder mode config
- **main.rs / shell init**: Return-to-ez loop after tmux detach
- **plugins/tmux/**: Investigate kill reliability, return-to-ez integration
- **README.md**: Refresh and polish
