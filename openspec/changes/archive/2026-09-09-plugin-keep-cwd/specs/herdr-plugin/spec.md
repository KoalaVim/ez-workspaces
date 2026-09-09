## MODIFIED Requirements

### Requirement: Keybind opens herdr workspace
On `on_bind`, the plugin SHALL open and focus a herdr workspace for the session path via `post_shell_commands`. When running inside herdr (`HERDR_PANE_ID` is set), the response SHALL include `accept: true` so the browser exits without relaunching, and `keep_cwd: true` so ez leaves the invoking shell's working directory alone — focus moves to the herdr workspace, not to this shell. When running outside herdr the response attaches herdr in the current terminal and SHALL NOT set `keep_cwd`.

#### Scenario: alt-h pressed inside herdr
- **WHEN** `on_bind` fires for the `herdr_open` bind and herdr is available and `HERDR_PANE_ID` is set
- **THEN** the plugin returns a `post_shell_commands` entry that opens the herdr workspace with `accept: true` and `keep_cwd: true`

#### Scenario: alt-h pressed outside herdr
- **WHEN** `on_bind` fires for the `herdr_open` bind and herdr is available and `HERDR_PANE_ID` is not set
- **THEN** the plugin returns a `post_shell_commands` entry that opens the herdr workspace and attaches herdr, without `accept` and without `keep_cwd`

#### Scenario: alt-h when herdr unavailable
- **WHEN** `on_bind` fires but herdr is not available
- **THEN** the plugin returns `{success: true}` and ez falls back to its default behavior

#### Scenario: on_create = herdr does not move the creating shell
- **WHEN** `on_bind` runs as the `on_create`/`on_enter` action from a shell inside herdr
- **THEN** `keep_cwd: true` keeps that shell in its original directory while the new session's workspace is focused
