## Context

The session list appears in two places: the interactive fzf browser (`session_action_loop` in `src/browser/mod.rs`) and the CLI `ez session list` command (`src/session/mod.rs`). Both currently append `→ <path>` after each session name, which duplicates what the preview pane already shows and adds visual clutter.

Currently there's no way to tell at a glance which sessions have an active multiplexer attachment (a tmux client connected, a zellij session running, or a herdr workspace open+focused). The session name is always rendered in yellow/bold.

Each multiplexer plugin already knows how to detect running sessions:
- **tmux**: `tmux list-sessions -F '#{session_name}|#{session_attached}|#{@ez_managed}'` — `session_attached > 0` means a client is connected.
- **zellij**: `zellij list-sessions -n` — sessions not marked `EXITED` are running; a session is "attached" if the current shell is inside it (`$ZELLIJ_SESSION_NAME`).
- **herdr**: `herdr workspace list` returns open workspaces with their checkout paths; `herdr worktree list --cwd <repo>` maps paths to workspace IDs.

## Goals / Non-Goals

**Goals:**
- Remove the `→ <path>` suffix from session items in both the interactive browser and CLI `session list` (tree and flat modes).
- Render attached session names in aqua/cyan instead of yellow to visually distinguish them.
- Keep detection fast — the browser re-renders on every action loop iteration.

**Non-Goals:**
- Changing the preview pane layout (path is already shown there).
- Distinguishing between multiplexer types in the color indicator (all attached sessions look the same regardless of tmux/zellij/herdr).

## Decisions

### 1. Remove path display from session items

Remove the `→ <path>` rendering in three locations:
- `src/browser/mod.rs` lines ~381-386 (interactive browser)
- `src/session/mod.rs` lines ~707-711 and ~729-734 (CLI `session list` flat and tree modes)

The path remains visible in the preview pane and via `--json` output.

### 2. Detect attached sessions via a new `OnAttachedSessions` plugin hook

**Why**: Multiplexer-specific detection logic already lives in the plugins (tmux, zellij, herdr). Each plugin knows its own session naming conventions, commands, and what "attached" means. Adding a plugin hook keeps that knowledge where it belongs and lets future or user-written plugins participate without touching Rust.

**Hook protocol**:
- New `HookType::OnAttachedSessions` variant.
- The request includes the repo info and the full session list (all sessions for this repo) so the plugin can match its state against known sessions.
- The response uses a new `attached_sessions` field on `HookResponse`: a `Vec<String>` of session IDs that the plugin considers attached.
- The Rust side calls `run_hooks` with `OnAttachedSessions`, collects `attached_sessions` from each plugin response, and unions them into a `HashSet<SessionId>`.

**Plugin implementations**:
- **tmux**: `tmux list-sessions -F '#{session_name}|#{session_attached}|#{@ez_managed}'` — filter for `@ez_managed=1` and `session_attached>0`, decode `<repo>/<session>` names, match against the session list from the request, return matching session IDs.
- **zellij**: `zellij list-sessions -n` — filter non-EXITED, match encoded mux names against sessions from the request, return matching session IDs.
- **herdr**: `herdr worktree list --cwd <repo>` — match `open_workspace_id` presence against session paths from the request, return matching session IDs.

**Alternative considered**: Querying multiplexers directly from Rust — rejected because it duplicates naming/encoding logic that plugins already own and prevents third-party plugins from reporting attached state.

### 3. Aqua color for attached sessions

Use `colored::Color::Cyan` (which renders as aqua in most terminals) for the session name when the session is in the attached set. Non-attached sessions keep the current `bold().yellow()` styling.

The color change applies only to the session name itself — tree connectors, markers, and labels keep their existing colors.

### 4. Cache attached state per render cycle

Call the `OnAttachedSessions` hook once per action-loop iteration (before building the `SelectItem` list), not per-session. `run_hooks` already batches all enabled plugins for a given hook type, so this is one call that fans out to each plugin.

For CLI `session list`, call it once before the print loop.

## Risks / Trade-offs

- **[Subprocess cost]** → Each render spawns up to 3 subprocesses (one per multiplexer). In practice only 1-2 are enabled. Each completes in <50ms. Acceptable for an interactive loop that already shells out for fzf.
- **[Race condition]** → A session could attach/detach between the query and the render. The next render cycle will correct it. Acceptable for a cosmetic indicator.
- **[herdr detection]** → herdr's `workspace list` doesn't directly report "attached" — it reports open workspaces. An open workspace with a focused window is the closest proxy. For simplicity, we'll treat any open herdr workspace as "attached". This matches user mental model (if it's open in herdr, it's active).
