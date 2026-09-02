# Plugin System

## MODIFIED Requirements

### Requirement: JSON-over-stdio protocol
The system SHALL communicate with plugins by spawning the executable, writing a JSON `HookRequest` to stdin, closing stdin, and reading a JSON `HookResponse` from stdout. Diagnostics go to stderr. On Windows, the system SHALL spawn plugins via `bash.exe <executable>` since the OS does not support shebang resolution. On Unix, the system SHALL spawn the executable directly.

#### Scenario: Plugin invocation
- **WHEN** a hook is triggered
- **THEN** system spawns the plugin executable, writes a JSON request to stdin with hook type, session/repo context, and user config, then reads the JSON response from stdout

#### Scenario: Plugin invocation on Windows
- **WHEN** a hook is triggered on Windows
- **THEN** system spawns `bash.exe` with the plugin script as argument, writes the JSON request to stdin, and reads the JSON response from stdout

#### Scenario: Plugin timeout
- **WHEN** a plugin does not respond within the configured `plugin_timeout` seconds
- **THEN** system returns `PluginTimeout` error
