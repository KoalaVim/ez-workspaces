## MODIFIED Requirements

### Requirement: On-enter and on-create actions
The config SHALL support `on_enter` (default "cd") and `on_create` (default "none") fields that control what happens when a session is entered or created. Values can be "cd", "none", or a plugin-bind label/name — including `"tmux"` and `"zellij"` for the bundled multiplexer plugins. These are overridable per-invocation via `--on-enter` and `--on-create` CLI flags.

#### Scenario: Override on CLI
- **WHEN** user runs `ez --on-enter tmux`
- **THEN** session enter action uses the tmux bind regardless of config

#### Scenario: Zellij as the enter action
- **WHEN** config sets `on_enter = "zellij"` and the zellij plugin is enabled
- **THEN** entering a session attaches to (or switches to) that session's zellij session instead of cd-ing

#### Scenario: Unavailable bind falls back to cd
- **WHEN** `on_enter = "zellij"` but the zellij plugin is disabled
- **THEN** ez falls back to `cd` without erroring

### Requirement: Plugin settings
The config SHALL support a `[plugin_settings.<name>]` section for per-plugin user-facing settings. These are passed to plugins as `config.user_config` in every hook request.

#### Scenario: Tmux settings
- **WHEN** config has `[plugin_settings.tmux] auto_attach = true`
- **THEN** the tmux plugin receives `{"auto_attach": true}` in its hook requests

#### Scenario: Zellij settings
- **WHEN** config has `[plugin_settings.zellij]` with `auto_attach`, `force_delete`, or `reap_delay_ms`
- **THEN** the zellij plugin receives those values in `config.user_config` in its hook requests

### Requirement: Browser loop configuration
The config SHALL support a `browser_loop` boolean field (default `true`) that controls whether the return-to-ez loop is active after detaching from a multiplexer session (tmux or zellij). This can also be overridden per-invocation via the `--no-loop` CLI flag.

#### Scenario: Disable loop via config
- **WHEN** config has `browser_loop = false`
- **THEN** the shell wrapper does not re-enter the browser after a multiplexer detach

#### Scenario: Override via CLI flag
- **WHEN** user runs `ez --no-loop`
- **THEN** the loop is disabled for this invocation regardless of config
