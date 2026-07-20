## Context

ez-workspaces manages workspace sessions with a plugin system where bundled plugins (git-worktree, tmux) hook into the session lifecycle. KoalaVim is managed by the `kv` CLI which supports virtual environments (`kv env`) — isolated copies of config, data, state, and cache. Currently all sessions share the default `main` kv env.

The existing bundled plugins provide a clear pattern: git-worktree creates worktrees on `on_session_create` and removes them on `on_session_delete`; tmux does the same for tmux sessions. The kv plugin follows this same pattern for kv environments.

## Goals / Non-Goals

**Goals:**
- Each ez session gets its own isolated kv environment (forked from `main`)
- Env variable `KV_ENV` is injected into the session so `kv` auto-selects the right env
- Lifecycle parity with other plugins: create, delete, enter, rename hooks
- Graceful degradation when `kv` is not installed

**Non-Goals:**
- Managing kv environments outside ez sessions (users can still use `kv env` directly)
- Auto-enabling the plugin (users who don't use KoalaVim shouldn't be affected)
- Syncing kv env config between sessions (each fork is independent after creation)
- Custom fork source (always forks from `main`; custom source is a future enhancement)

## Decisions

### 1. Bundled bash plugin (same as git-worktree and tmux)

The plugin is a bash script shipped inside the ez binary, following the exact pattern of existing bundled plugins. This keeps the implementation simple, consistent, and easy to maintain.

**Alternative**: Rust-native integration inside ez core. Rejected because kv is an external tool with its own CLI — shelling out via the plugin system is the right abstraction boundary.

### 2. Env name = session name (no repo prefix)

The kv env name matches the session name directly (e.g., session `feature-auth` → kv env `feature-auth`). No repo prefix because kv envs are global (not per-repo) and session names are already user-facing identifiers.

**Alternative**: `<repo>/<session>` naming (like tmux plugin). Rejected because kv env names don't support `/` and adding separators would create mismatches with what users see in `kv env list`.

### 3. Fork from `main` env on create

On `on_session_create`, the plugin runs `kv env fork main <session-name>`. The `main` env is the default kv environment and serves as the baseline config. Default sessions (is_default=true) skip the fork — they use the existing `main` env directly.

**Alternative**: `kv env create` (empty env). Rejected because users want their base config/plugins carried over — fork is the right semantic.

### 4. KV_ENV injection via session env mutations

The plugin sets `KV_ENV=<env-name>` in `session_mutations.env`. This env var is picked up by the shell wrapper and propagated to tmux (if tmux plugin is also active). `kv` reads `KV_ENV` to determine which environment to use.

### 5. Configurable source env

A `source_env` config option (default: `"main"`) lets users choose which kv env to fork from. Most users fork from `main`, but power users may want a different base.

## Risks / Trade-offs

- **[Risk] kv not installed** → Plugin checks `command -v kv` at the top of each hook. If missing, returns `{"success": true}` (no-op). Create hook returns success so session creation isn't blocked.
- **[Risk] Fork fails (e.g., source env doesn't exist)** → `on_session_create` returns `{"success": false, "error": ...}` which aborts session creation. User gets a clear error message.
- **[Risk] Orphaned kv envs** → If a session is deleted outside ez (e.g., manual sessions.toml edit), the kv env lingers. Users can clean up with `kv env delete`. This matches how orphaned tmux sessions and worktrees are handled.
- **[Risk] Env name collision** → If a kv env with that name already exists (created manually or from another repo), the plugin reuses it rather than failing. This is the same behavior as tmux — we don't own the namespace exclusively.
- **[Trade-off] Disk usage** → Each forked env duplicates kv state/cache. Acceptable because envs are lightweight and the isolation benefit outweighs storage cost.
