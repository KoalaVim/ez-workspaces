## ADDED Requirements

### Requirement: Zellij session naming
The plugin SHALL derive a zellij session name from the repo and ez session as `<repo-basename>__<session-name>`, where every character outside `[A-Za-z0-9_-]` in each part is replaced with `_`. The encoding SHALL be deterministic so that the name can be recomputed (and reverse-matched) from registered repo/session metadata without persisted state. Names too long for zellij's IPC socket path are shortened by the rule in "Session names fit the IPC socket path", which preserves that property.

#### Scenario: Name encoding
- **WHEN** repo `/Users/me/work/my.repo` has session `feat/ABC-1`
- **THEN** the zellij session name is `my_repo__feat_ABC-1`

#### Scenario: Slash is never emitted
- **WHEN** a repo basename or session name contains `/`
- **THEN** the encoded name contains `_` in its place, because zellij rejects session names containing `/`

### Requirement: Create zellij session on session create
On `OnSessionCreate`, when `zellij` is on `PATH`, the plugin SHALL create a detached background zellij session named by the encoding rule, rooted at the ez session's path, with the session's `env` entries exported into the zellij server process. The plugin SHALL return `session_mutations` setting `env.EZ_ZELLIJ_SESSION` and `plugin_state.zellij_session` to the encoded name.

#### Scenario: Background session created at session path
- **WHEN** a session is created with path `/Users/me/work/repo-feat`
- **THEN** the plugin runs the zellij background-create command with that directory as the working directory
- **AND** the new zellij session is not attached to any client

#### Scenario: Session already exists
- **WHEN** a zellij session with the encoded name is already running
- **THEN** the plugin does not create a second session and still returns success

#### Scenario: Session env exported at creation
- **WHEN** the ez session has `env` entries (e.g. from the kv plugin)
- **THEN** those variables are present in the environment of the created zellij session's panes

### Requirement: Attach bind
The plugin SHALL register an `OnBind` keybind (`Alt-z`, name `zellij_attach`, label `zellij`) in the `session` context that ensures the zellij session exists and then returns `post_shell_commands` that place the user in it. When the user is already inside a zellij session (`$ZELLIJ` set), the command SHALL switch the current client to the target session; otherwise it SHALL attach in the current terminal.

#### Scenario: Attach from a plain shell
- **WHEN** user presses `Alt-z` on a session while not inside zellij
- **THEN** the plugin returns a post-shell command that attaches to the encoded session, creating it if missing

#### Scenario: Switch from inside zellij
- **WHEN** user presses `Alt-z` on a session while inside a zellij session
- **THEN** the plugin ensures the target session exists (created at the ez session's path) and returns a post-shell command that switches the current client to it

#### Scenario: Bind resolves on_enter and on_create
- **WHEN** config sets `on_enter = "zellij"` or `on_create = "zellij"`
- **THEN** the bind's label resolves through the existing bind-label lookup and the same attach/switch behavior runs on session enter or create

### Requirement: Auto-attach on session enter
The plugin SHALL support an `auto_attach` boolean setting (default `false`). On `OnSessionEnter` with `auto_attach = true`, the plugin SHALL return the same attach/switch post-shell command as the attach bind. With `auto_attach = false` it SHALL return success with no commands.

#### Scenario: Auto-attach enabled
- **WHEN** `[plugin_settings.zellij] auto_attach = true` and the user enters a session
- **THEN** the plugin returns a post-shell command attaching to (or switching to) the session's zellij session

#### Scenario: Auto-attach disabled
- **WHEN** `auto_attach` is unset or `false` and the user enters a session
- **THEN** the plugin returns `{"success": true}` with no post-shell commands

### Requirement: Rename propagation
On `OnSessionRename`, the plugin SHALL rename the running zellij session from the old encoded name to the new encoded name. If no zellij session with the old name is running, the plugin SHALL succeed without error.

#### Scenario: Running session renamed
- **WHEN** session `old-name` is renamed to `new-name` and its zellij session is running
- **THEN** the plugin renames the zellij session to the newly encoded name

#### Scenario: No running session
- **WHEN** the session being renamed has no running zellij session
- **THEN** the plugin returns success and makes no zellij calls that fail the hook

### Requirement: Teardown on session delete
On `OnSessionDelete`, the plugin SHALL terminate the session's zellij session. When the `force_delete` setting is `true` (default), it SHALL also remove zellij's serialized/resurrectable copy of that session so the name does not linger in `zellij list-sessions`. Failures SHALL be swallowed so the hook always reports success.

#### Scenario: Running session killed and deleted
- **WHEN** an ez session with a running zellij session is deleted and `force_delete = true`
- **THEN** the zellij session is killed and its serialized state deleted, leaving no entry in `zellij list-sessions`

#### Scenario: Kill only
- **WHEN** `force_delete = false`
- **THEN** the plugin kills the running session but leaves zellij's resurrectable entry intact

#### Scenario: Missing session tolerated
- **WHEN** no zellij session exists for the deleted ez session
- **THEN** the hook still returns `{"success": true}`

### Requirement: Zellij browser view
The plugin SHALL register a view (`Ctrl-z`, name `zellij`, label `zellij`) in the `repo`, `owner`, `workspace`, `tree`, and `label` contexts. `OnView` SHALL list every session of every registered repo, marking each with `●` when its encoded zellij session is currently running and `○` when it is not. `OnViewSelect` SHALL attach to (or switch to) the selected session, creating the zellij session at the ez session's path when it is not running.

#### Scenario: View lists all sessions with running markers
- **WHEN** user presses `Ctrl-z` in the browser
- **THEN** the view lists `<repo>/<session>` entries for all registered repos, with `●` for sessions whose zellij session appears in the running session list

#### Scenario: Select a stopped session
- **WHEN** user selects an entry whose zellij session is not running
- **THEN** the plugin creates the zellij session at that ez session's path and returns a post-shell command that attaches to or switches to it

#### Scenario: Select a running session
- **WHEN** user selects an entry whose zellij session is running
- **THEN** the plugin returns a post-shell command that attaches to or switches to the existing session without recreating it

### Requirement: Graceful degradation without zellij
Every hook SHALL check for the `zellij` executable before invoking it. Lifecycle hooks (`OnSessionCreate`, `OnSessionDelete`, `OnSessionEnter`, `OnSessionExit`, `OnSessionRename`) SHALL return success and do nothing when zellij is absent; `OnBind` SHALL return an error message stating zellij is not installed; `OnView` SHALL return an empty item list with a prompt indicating zellij is not installed.

#### Scenario: Lifecycle hook without zellij
- **WHEN** zellij is not on `PATH` and a session is created
- **THEN** the plugin returns `{"success": true}` and session creation completes normally

#### Scenario: Bind without zellij
- **WHEN** zellij is not on `PATH` and the user presses `Alt-z`
- **THEN** the plugin returns `success: false` with an error stating that zellij is not installed

### Requirement: Zellij session creation failures are reported, never assumed
The plugin SHALL treat a failed zellij session creation as a failure rather than proceeding as if the session exists. `OnBind` and `OnViewSelect` SHALL return `success: false` carrying zellij's own error output and SHALL NOT return a `post_shell_commands` attach command. Lifecycle hooks (`OnSessionCreate`, `OnSessionEnter`) SHALL write the error to stderr and still return success, so a multiplexer failure never aborts the ez operation, and `OnSessionEnter` SHALL omit the attach command in that case.

#### Scenario: Create failure during session create
- **WHEN** the zellij session cannot be created while an ez session is being created
- **THEN** the hook returns `{"success": true}` with its normal `session_mutations` and writes the zellij error to stderr

### Requirement: Session names fit the IPC socket path
Because a session's IPC socket is named after the session, a name whose socket path exceeds the platform limit (103 bytes) is unusable — and a name that fits only a *shorter* path than the one in effect is worse than unusable, since every other zellij process (a plain `zellij` command, the built-in session manager, the server hosting another session) rebuilds the path from its own environment and can neither list, attach to, nor delete the resulting session.

The plugin SHALL therefore compute the bytes available for a name as `103 - len(socket directory) - len(contract-version directory) - 2`, and SHALL derive every zellij session name so that it fits that budget:

- a name that fits SHALL be used verbatim as `<repo>__<session>`;
- a name that does not SHALL become `<encoded session name, truncated to fit>_<first 4 hex digits of the md5 of the full encoded name>`.

The contract-version directory name SHALL be read from the socket directory when present, and otherwise assumed to be the widest two-digit form, so the budget is never an overestimate. Current-session detection SHALL accept both forms, and SHALL do so without re-deriving the budget, so a session created under a different `$TMPDIR` or `socket_dir` still resolves.

The plugin SHALL use zellij's own default socket directory unless `socket_dir` is set or `ZELLIJ_SOCKET_DIR` is already exported, so that ez's sessions share one namespace with sessions started outside ez. A configured directory SHALL be exported to every zellij invocation and to the attach command handed to the user's shell.

#### Scenario: Long name is shortened to a reachable one
- **WHEN** a session encodes to `hypersonic__type-aware-lint` (27 bytes) on a macOS host whose default socket path leaves 24 bytes for a name
- **THEN** the plugin creates the zellij session as `type-aware-lint_9db7` in zellij's default socket directory
- **AND** that session is listed by a plain `zellij list-sessions` and attachable from any shell, with no `ZELLIJ_SOCKET_DIR` in the attach command

#### Scenario: Short name is left alone
- **WHEN** the encoded name fits the budget, as `hypersonic__main` does
- **THEN** the plugin uses it verbatim

#### Scenario: Shortened names stay distinct
- **WHEN** two repos have a session with the same name, or two long session names in one repo share a truncated prefix
- **THEN** their digests differ, because the digest covers the full `<repo>__<session>` name

#### Scenario: Current session resolves from either form
- **WHEN** `$ZELLIJ_SESSION_NAME` is a shortened name such as `type-aware-lint_9db7`
- **THEN** current-session detection resolves it to the ez session whose full encoded name has that digest

#### Scenario: Explicit socket directory keeps full names
- **WHEN** `socket_dir` is set to a short directory, or `ZELLIJ_SOCKET_DIR` is already exported
- **THEN** the plugin uses that directory, the budget grows accordingly, and names that fit it are used verbatim

#### Scenario: No room for any name
- **WHEN** the socket directory is so long that fewer than 6 bytes remain for a name
- **THEN** no zellij session is created and the reported error advises setting a shorter `socket_dir`

### Requirement: Plugin configuration schema
The plugin manifest SHALL declare the settings `auto_attach` (bool, default `false`), `force_delete` (bool, default `true`), `socket_dir` (string, default `""`), and `reap_delay_ms` (int, default `200`), read from `[plugin_settings.zellij]`.

#### Scenario: Settings delivered to the plugin
- **WHEN** config contains `[plugin_settings.zellij] auto_attach = true`
- **THEN** the plugin receives `auto_attach = true` in `config.user_config` on every hook request

### Requirement: Coexistence with the tmux plugin
The zellij plugin SHALL use keys distinct from the tmux plugin (`Alt-z` / `Ctrl-z` versus `Alt-a` / `Ctrl-a`) so both plugins can be enabled simultaneously without keybind collisions.

#### Scenario: Both plugins enabled
- **WHEN** both the tmux and zellij plugins are enabled
- **THEN** the browser exposes `Alt-a`/`Ctrl-a` for tmux and `Alt-z`/`Ctrl-z` for zellij, and each bind targets only its own multiplexer

### Requirement: Environment changes require session recreation
Because zellij provides no way to mutate the environment of a running session, the plugin SHALL apply session `env` only when creating a zellij session. Changes to a session's env after its zellij session exists SHALL NOT be reflected in that running session.

#### Scenario: Env change after creation
- **WHEN** a session's `env` changes while its zellij session is already running
- **THEN** the running zellij session keeps the environment captured at creation time, and the new values apply only after the zellij session is recreated
