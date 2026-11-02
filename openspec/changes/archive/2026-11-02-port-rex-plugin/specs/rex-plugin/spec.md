## Purpose

Bundled plugin that maps ez onto the Rex multiplexer: one Rex session per repo, one Rex tab (window) per ez session, opened on demand and cleaned up when the ez session goes away.

## ADDED Requirements

### Requirement: Rex plugin manifest
The rex plugin SHALL ship as a bundled plugin with a `manifest.toml` named `rex`, hooks `on_bind`, `on_session_delete` and `on_session_rename`, executable `rex-plugin`, and one keybind `alt-x` named `rex_open` labelled `rex` in the `session` context.

#### Scenario: Bundled plugin is extracted
- **WHEN** ez starts and extracts bundled plugins
- **THEN** `<config_dir>/ez/plugins/rex/manifest.toml` and an executable `rex-plugin` exist and the plugin can be enabled with `rex` in `plugins.enabled`

#### Scenario: Bind is usable as an on_enter/on_create action
- **WHEN** the config sets `on_enter = "rex"` or `on_create = "rex"`
- **THEN** entering or creating a session runs the `rex_open` bind

### Requirement: Graceful degradation when Rex is unavailable
When the `rex` CLI is not on PATH, every hook SHALL return `{"success": true}` and take no action. Hooks that only clean up (`on_session_delete`, `on_session_rename`) SHALL NOT start a Rex server that is not already running.

#### Scenario: rex not installed
- **WHEN** any hook fires and `rex` is not on PATH
- **THEN** the plugin returns `{"success": true}` with no post-shell commands

#### Scenario: Rex server not running during delete
- **WHEN** `on_session_delete` fires and no Rex server is running
- **THEN** the plugin returns `{"success": true}` and no Rex server is started

### Requirement: Hooks always return valid JSON
Every hook invocation SHALL write a single valid JSON response with `success: true` to stdout, even if a `rex` or `jq` call fails partway through. Rex failures SHALL NOT fail the ez operation (create, enter, delete, rename).

#### Scenario: rex call fails mid-hook
- **WHEN** a `rex` command exits non-zero or prints non-JSON during `on_session_delete` or `on_session_rename`
- **THEN** the plugin still prints `{"success": true}` and ez completes the operation

### Requirement: Bind opens and focuses the session's tab
On `on_bind` for `rex_open`, the plugin SHALL return a single post-shell command that, when run, finds or creates the Rex session labelled with the repo name (cwd: session path), finds or creates the tab in it labelled with the session name (cwd: session path), and switches Rex to that tab. The response SHALL set `accept: true` and `keep_cwd: true`. The command SHALL be correctly shell-quoted for paths and names containing spaces, slashes, or other shell metacharacters.

#### Scenario: First open of a session
- **WHEN** `alt-x` is pressed on session `feat/x` of repo `app` and Rex has no session labelled `app`
- **THEN** a Rex session `app` is created with one tab `feat/x` in the session's directory and Rex switches to it

#### Scenario: Labels match case-insensitively
- **WHEN** Rex already has a session labelled `App` and `alt-x` is pressed on a session of repo `app`
- **THEN** the existing `App` Rex session is reused rather than a second one created, matching how Rex itself resolves label targets

#### Scenario: Re-open is idempotent
- **WHEN** `alt-x` is pressed again on the same session
- **THEN** no new Rex session or tab is created and Rex switches to the existing tab

#### Scenario: Plugin path contains a space
- **WHEN** the plugin is installed under a directory containing a space (e.g. `~/Library/Application Support/ez/plugins/rex`)
- **THEN** the post-shell command still runs the plugin's focus step

#### Scenario: Switching fails
- **WHEN** the tab exists but Rex refuses to switch to it (e.g. Remote Control disabled)
- **THEN** the focus step prints an error naming the repo/session and the Rex error to stderr and exits non-zero; when `REX_EZ_POPUP` is set it waits for a key press before exiting

### Requirement: Session delete closes its tab
On `on_session_delete`, the plugin SHALL close the tab labelled with the session name in the Rex session labelled with the repo name, unless that tab contains the pane ez is running in (`REX_BLOCK`). If closing leaves the Rex session with no tabs, the plugin SHALL kill that Rex session.

#### Scenario: Tab closed on delete
- **WHEN** session `feat/x` of repo `app` is deleted from outside its tab
- **THEN** the `feat/x` tab is closed

#### Scenario: Last tab closed
- **WHEN** the deleted session's tab was the only tab in Rex session `app`
- **THEN** Rex session `app` is killed

#### Scenario: ez runs inside the tab being deleted
- **WHEN** the tab to close contains the block in `REX_BLOCK`
- **THEN** the tab is left open and the delete still succeeds

#### Scenario: No tab exists
- **WHEN** the session was never opened in Rex
- **THEN** nothing is closed and the plugin returns success

### Requirement: Session rename relabels its tab
On `on_session_rename`, the plugin SHALL rename the tab labelled with the old session name in the repo's Rex session to the new session name, so a later `alt-x` focuses it instead of creating a duplicate.

#### Scenario: Rename then re-open
- **WHEN** session `feat/x` with an open tab is renamed to `feat/y` and `alt-x` is pressed on it
- **THEN** the existing tab (now labelled `feat/y`) is focused and no new tab is created

#### Scenario: Rename with no tab
- **WHEN** a session that has no Rex tab is renamed
- **THEN** the plugin returns success and changes nothing in Rex
