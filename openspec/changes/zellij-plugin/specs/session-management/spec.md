## MODIFIED Requirements

### Requirement: Current session detection
The system SHALL detect the current session from, in order: (1) the tmux `@ez_repo_id` + `@ez_session_name` user options, (2) the tmux `@ez_session_path` user option, (3) the `$ZELLIJ_SESSION_NAME` environment variable matched against the encoded `<repo-basename>__<session-name>` name of every registered session, or (4) matching the current working directory against known session worktree paths. The first source that resolves to a registered session wins.

#### Scenario: Detect from tmux
- **WHEN** user is inside a tmux session managed by ez
- **THEN** system reads the tmux user options to identify the current session

#### Scenario: Detect from zellij
- **WHEN** user is inside a zellij session whose name matches the encoded name of a registered session
- **THEN** system identifies that session as the current one without requiring any persisted plugin state

#### Scenario: Zellij session name does not match
- **WHEN** `$ZELLIJ_SESSION_NAME` is set but matches no registered session (e.g. a hand-made zellij session)
- **THEN** system falls through to working-directory matching

#### Scenario: Detect from worktree path
- **WHEN** user is inside a directory that matches a registered session's worktree path
- **THEN** system identifies the current session by path matching

### Requirement: Session tree hierarchy
Sessions SHALL be organized in a tree using `parent_id` pointers. The system SHALL support operations: list roots, find children, find ancestors, find descendants, and render as an indented tree. The `session list` command SHALL support an `--all` flag that covers every registered repo instead of a single repo.

#### Scenario: Render tree
- **WHEN** user runs `ez session list`
- **THEN** system renders sessions as an indented tree with root sessions at the top level and children indented below their parents

#### Scenario: Flat list
- **WHEN** user runs `ez session list --flat`
- **THEN** system renders sessions as a flat list without tree structure

#### Scenario: JSON session list
- **WHEN** user runs `ez session list --json --repo my-repo`
- **THEN** system outputs a JSON array of session objects with fields: `id`, `name`, `parent_id`, `path`, `bare`, `labels`, `last_accessed`, `env`, `is_default`

#### Scenario: All-repo JSON listing
- **WHEN** user runs `ez session list --all --json`
- **THEN** system outputs a JSON array of objects, one per registered repo, each with the repo's `id`, `name`, `path`, and a `sessions` array using the same session fields as the single-repo JSON output

#### Scenario: All flag with repo flag
- **WHEN** user runs `ez session list --all --repo my-repo`
- **THEN** system reports an error that `--all` and `--repo` are mutually exclusive

## ADDED Requirements

### Requirement: Detached delete reaper isolation
The detached worker that runs `OnSessionDelete` hooks SHALL be started with the multiplexer environment variables cleared (`TMUX`, `TMUX_PANE`, `ZELLIJ`, `ZELLIJ_SESSION_NAME`, `ZELLIJ_PANE_ID`) so that hooks tearing down the multiplexer session the user is attached to cannot mistake the worker for a client inside that session.

#### Scenario: Deleting the session you are attached to
- **WHEN** user deletes the ez session whose zellij session they are currently attached to
- **THEN** the reaper worker runs outside that session's environment, kills the zellij session, and completes even though the user's client is disconnected

### Requirement: Multiplexer reap delay setting
The reaper delay SHALL be read from `[plugin_settings.tmux] reap_delay_ms`, falling back to `[plugin_settings.zellij] reap_delay_ms`, and defaulting to 200 milliseconds when neither is set.

#### Scenario: Zellij-only configuration
- **WHEN** config sets `[plugin_settings.zellij] reap_delay_ms = 500` and has no tmux settings
- **THEN** the reaper waits 500 ms before running delete hooks

#### Scenario: Neither configured
- **WHEN** neither plugin declares `reap_delay_ms`
- **THEN** the reaper waits 200 ms
