# KoalaVim (kv) Environment Plugin

## Purpose

Provide per-session KoalaVim environment isolation via the `kv` CLI. Each session gets its own forked kv env so the editor has a separate config, cache, and state directory. The plugin manages the full env lifecycle (fork, delete, rename) and sets `KV_ENV` so `kv` loads the correct environment. Requires explicit opt-in and repo-scoped activation.

## Requirements

### Requirement: Fork kv env on session create
The plugin SHALL fork the configured source kv env (default `main`) into a session-specific env when `on_session_create` fires. The env name SHALL be the session name. If `kv` is not installed, the hook SHALL return success (no-op). Default sessions (is_default=true) SHALL be skipped.

#### Scenario: Create session forks kv env
- **WHEN** a non-default session named `feature-auth` is created and `kv` is installed
- **THEN** the plugin runs `kv env fork main feature-auth`
- **AND** sets `KV_ENV=feature-auth` in `session_mutations.env`
- **AND** stores `kv_env_name: "feature-auth"` in `session_mutations.plugin_state`

#### Scenario: Default session skipped
- **WHEN** a default session (is_default=true) is created
- **THEN** the plugin returns success without forking (the default session uses the `main` kv env)

#### Scenario: kv not installed
- **WHEN** `kv` is not found in PATH
- **THEN** the plugin returns `{"success": true}` without error

#### Scenario: kv env already exists
- **WHEN** a kv env with the session name already exists
- **THEN** the plugin reuses the existing env (sets `KV_ENV` without forking)

#### Scenario: Fork failure
- **WHEN** `kv env fork` fails (e.g., source env not found)
- **THEN** the plugin returns `{"success": false, "error": "<message>"}` aborting session creation

### Requirement: Delete kv env on session delete
The plugin SHALL delete the session's kv env when `on_session_delete` fires. Default sessions SHALL be skipped.

#### Scenario: Delete session removes kv env
- **WHEN** a non-default session named `feature-auth` is deleted
- **THEN** the plugin runs `kv env delete --force feature-auth`

#### Scenario: Default session skipped on delete
- **WHEN** the default session is deleted
- **THEN** the plugin returns success without deleting any kv env

#### Scenario: kv env doesn't exist on delete
- **WHEN** the session's kv env was already deleted or never created
- **THEN** the plugin returns success (no error)

### Requirement: Set KV_ENV on session enter
The plugin SHALL set the `KV_ENV` environment variable to the session's env name on `on_session_enter`. Default sessions SHALL set `KV_ENV=main`.

#### Scenario: Enter session sets KV_ENV
- **WHEN** user enters session `feature-auth`
- **THEN** the plugin returns `session_mutations.env` with `KV_ENV=feature-auth`

#### Scenario: Enter default session
- **WHEN** user enters the default (main) session
- **THEN** the plugin returns `session_mutations.env` with `KV_ENV=main`

### Requirement: Rename kv env on session rename
The plugin SHALL rename the session's kv env when `on_session_rename` fires, using the `rename_context` to get old and new names.

#### Scenario: Rename session renames kv env
- **WHEN** session `feature-auth` is renamed to `feature-login`
- **THEN** the plugin runs `kv env rename feature-auth feature-login`
- **AND** updates `KV_ENV` to `feature-login` in session env

#### Scenario: kv env rename failure
- **WHEN** `kv env rename` fails
- **THEN** the plugin returns success (rename errors are non-fatal warnings)

#### Scenario: Default session rename skipped
- **WHEN** the default session is renamed
- **THEN** the plugin returns success without renaming (default maps to `main`)

### Requirement: Session path mutation
The plugin SHALL set `mutates_session_path = true` and `priority = 10` so it runs before the git-worktree plugin. On session create, it SHALL set `session_mutations.path` to the kv-managed KoalaVim directory (`kv env path <name>`) so the session points at the kv worktree instead of a git-worktree-managed one.

#### Scenario: Session path set to kv directory
- **WHEN** the kv plugin creates a session env
- **THEN** it runs `kv env path <session-name>` and sets `session_mutations.path` to the result
- **AND** the git-worktree plugin sees the path is already set and does branch checkout only

### Requirement: Repo-scoped activation
The plugin SHALL only activate for repos listed in the `repos` config field (comma-separated directory names). If `repos` is empty, the plugin skips all repos.

#### Scenario: Matching repo
- **WHEN** `repos = "KoalaVim, my-editor"` and the current repo basename is `KoalaVim`
- **THEN** the plugin runs normally

#### Scenario: Non-matching repo
- **WHEN** `repos = "KoalaVim"` and the current repo basename is `my-app`
- **THEN** the plugin returns success (no-op)

### Requirement: Plugin manifest and config
The plugin SHALL declare hooks `on_session_create`, `on_session_delete`, `on_session_enter`, and `on_session_rename` in its manifest. It SHALL expose a `source_env` config option (type string, default `"main"`) and a `repos` config option (type string, default `""`) for repo-scoped activation.

#### Scenario: Manifest declares hooks
- **WHEN** the plugin manifest is loaded
- **THEN** it declares hooks for session create, delete, enter, and rename

#### Scenario: Custom source env
- **WHEN** user sets `[plugin_settings.kv] source_env = "dev"` in config.toml
- **THEN** the plugin forks from `dev` instead of `main` on session create

### Requirement: Bundled plugin
The kv plugin SHALL be bundled in the ez binary and auto-extracted like git-worktree and tmux. It SHALL NOT be auto-enabled — users must explicitly run `ez plugin enable kv`.

#### Scenario: First-run extraction
- **WHEN** user runs `ez plugin enable kv` and the plugin files don't exist
- **THEN** system extracts the bundled plugin to `~/.config/ez/plugins/kv/`

#### Scenario: Auto-update on version change
- **WHEN** the bundled kv plugin version differs from the installed version
- **THEN** system overwrites the installed plugin files with the new version
