## Context

Sessions already store an `env: HashMap<String, String>` field, persisted in `sessions.toml`. Today this map is only writable by plugins via `SessionMutations.env` in hook responses. The `OnSessionEnter` hook (in the tmux plugin) exports these vars into the shell environment. Users have no way to manually set, inspect, or remove env vars without writing a plugin.

The existing `SessionCommand` enum in `cli.rs` already follows the nested-subcommand pattern for `Label` and `Note`. We'll add an `Env` subcommand group following the same pattern.

## Goals / Non-Goals

**Goals:**
- Let users set/unset/list env vars on any session via CLI
- Follow existing CLI patterns (`ez session env set`, `ez session env list`, `ez session env unset`)
- Env vars set via CLI are indistinguishable from plugin-set ones (same storage, same export on enter)

**Non-Goals:**
- No `env export` command (shell export is already handled by the tmux/shell-integration plugins on enter)
- No validation of env var names/values beyond basic non-empty key check
- No scoped env vars (all env vars are global to the session)
- No `--env` flag on `ez session new` (can be added later)

## Decisions

### 1. Command structure: nested subcommand under `session`

`ez session env {set|list|unset}` — mirrors the `label` and `note` subcommand groups.

Alternative considered: top-level `ez env set` — rejected because env vars are inherently per-session, the nesting communicates ownership.

### 2. Session resolution: `--session` flag with auto-detect fallback

All three commands accept `--session <name>` (optional) and `--repo <name|path>` (optional). If `--session` is omitted, the system uses current-session detection (tmux env → worktree path matching). If detection fails, error with guidance.

Alternative considered: positional session arg — rejected because it conflicts with `set KEY VALUE` positional args and is inconsistent with how `label` commands work.

### 3. Set semantics: upsert

`ez session env set KEY VALUE` inserts or overwrites. No distinction between create and update. This matches how `HashMap::insert` works and avoids unnecessary complexity.

### 4. Unset semantics: remove silently

`ez session env unset KEY` removes the key. If the key doesn't exist, succeed silently (idempotent). This follows the principle of least surprise for scripting.

### 5. List output: `KEY=VALUE` lines, with `--json` option

Default output is one `KEY=VALUE` per line (suitable for piping to `export`). `--json` outputs a JSON object. Colored output: key in cyan, `=` dimmed, value in default.

### 6. Implementation location: `src/session/mod.rs`

Add three functions (`session_env_set`, `session_env_list`, `session_env_unset`) in the session module alongside existing session operations. They load the session tree, mutate the target session's env map, and save.

## Risks / Trade-offs

- **[Risk] Conflict with plugin-managed keys** → Users can overwrite keys set by plugins (e.g. `ez_pr_number`). Mitigation: document that `ez_`-prefixed keys are managed by the system. No enforcement — power users may legitimately need to override.
- **[Risk] No env var name validation** → Users could set keys with spaces or special chars. Mitigation: shells won't export invalid names, so this is self-correcting. A future enhancement could warn on non-POSIX names.
- **[Trade-off] No `--env` on `session new`** → Keeps scope small. Can be added as a follow-up without breaking changes.
