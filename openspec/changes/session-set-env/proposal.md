## Why

Session env vars are currently only settable by plugins (via `SessionMutations.env`). Users have no direct CLI command to set or inspect env vars on a session, making it impossible to manually attach environment context (e.g. `AWS_PROFILE`, `DATABASE_URL`, feature flags) to a session without writing a plugin.

## What Changes

- Add `ez session env set <key> <value>` command to set (or update) an env var on a session
- Add `ez session env list` command to list all env vars on a session
- Add `ez session env unset <key>` command to remove an env var from a session

## Capabilities

### New Capabilities
- `session-env-cli`: CLI commands for managing per-session environment variables (set, list, unset)

### Modified Capabilities
- `session-management`: Add env management requirement to the session spec (env vars are already stored but only plugin-settable)

## Impact

- **Code**: New `Env` subcommand variant under `SessionCommand` in `cli.rs`, new handler functions in `src/session/mod.rs`
- **Existing behavior**: No breaking changes — plugin-set env vars continue to work identically
- **Docs**: `user-guide.md`, `README.md`, `AGENTS.md` need updates documenting the new commands
