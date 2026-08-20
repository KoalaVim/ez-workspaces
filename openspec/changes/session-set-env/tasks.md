## 1. CLI Definition

- [x] 1.1 Add `SessionEnvCommand` enum to `src/cli.rs` with `Set { key, value, session, repo }`, `List { session, repo, json }`, `Unset { key, session, repo }` variants
- [x] 1.2 Add `Env { command: SessionEnvCommand }` variant to `SessionCommand` enum

## 2. Core Implementation

- [x] 2.1 Add `session_env_set(repo, session_name, key, value)` function in `src/session/mod.rs` — loads tree, finds session, inserts key-value, saves
- [x] 2.2 Add `session_env_unset(repo, session_name, key)` function — loads tree, finds session, removes key, saves
- [x] 2.3 Add `session_env_list(repo, session_name, json_flag)` function — loads tree, finds session, prints env map

## 3. CLI Dispatch

- [x] 3.1 Wire `SessionEnvCommand` dispatch in `src/session/mod.rs` — match on `Set`/`List`/`Unset` and call the corresponding session functions
- [x] 3.2 Add current-session auto-detection fallback when `--session` is omitted (reuse `detect_current_session`)

## 4. Output & Error Handling

- [x] 4.1 Colored output for `list` (cyan key, dimmed `=`, default value) and confirmation messages for `set`/`unset`
- [x] 4.2 Error on empty key for `set`, error when no session detected and `--session` not provided

## 5. Documentation

- [x] 5.1 Update `docs/user-guide.md` with `session env` commands section
- [x] 5.2 Update `README.md` CLI reference
- [x] 5.3 Update `AGENTS.md` session module description to mention env commands
