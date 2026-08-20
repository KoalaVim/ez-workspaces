## ADDED Requirements

### Requirement: Set session environment variable
The system SHALL allow users to set an environment variable on a session via `ez session env set <KEY> <VALUE>`. The key SHALL be a non-empty string. The value SHALL be any string (including empty). If the key already exists, the system SHALL overwrite it. The command SHALL accept `--session <name>` to target a specific session (default: auto-detect current session) and `--repo <name|path>` to specify the repo (default: current repo).

#### Scenario: Set a new env var
- **WHEN** user runs `ez session env set AWS_PROFILE staging --session feature-x`
- **THEN** system sets `AWS_PROFILE=staging` in the session's env map and persists to disk
- **THEN** system prints confirmation: `Set AWS_PROFILE on session feature-x`

#### Scenario: Overwrite existing env var
- **WHEN** user runs `ez session env set AWS_PROFILE production` on a session that already has `AWS_PROFILE=staging`
- **THEN** system updates the value to `production` and persists

#### Scenario: Auto-detect current session
- **WHEN** user runs `ez session env set MY_VAR hello` without `--session` while inside a registered session
- **THEN** system detects the current session and sets the var on it

#### Scenario: No current session detected
- **WHEN** user runs `ez session env set MY_VAR hello` without `--session` and no current session can be detected
- **THEN** system returns an error with guidance to use `--session`

#### Scenario: Empty key rejected
- **WHEN** user runs `ez session env set "" value`
- **THEN** system returns an error "Environment variable key cannot be empty"

### Requirement: List session environment variables
The system SHALL allow users to list all environment variables on a session via `ez session env list`. Output SHALL be one `KEY=VALUE` per line with colored formatting (key in cyan, `=` dimmed). If `--json` is passed, output SHALL be a JSON object. The command SHALL accept `--session <name>` and `--repo <name|path>` with the same defaults as `set`.

#### Scenario: List env vars
- **WHEN** user runs `ez session env list --session feature-x`
- **THEN** system prints each env var as `KEY=VALUE`, one per line

#### Scenario: List with JSON output
- **WHEN** user runs `ez session env list --session feature-x --json`
- **THEN** system prints a JSON object mapping keys to values

#### Scenario: No env vars set
- **WHEN** user runs `ez session env list` on a session with no env vars
- **THEN** system prints nothing (empty output, exit 0)

#### Scenario: Auto-detect current session for list
- **WHEN** user runs `ez session env list` without `--session` while inside a registered session
- **THEN** system detects the current session and lists its env vars

### Requirement: Unset session environment variable
The system SHALL allow users to remove an environment variable from a session via `ez session env unset <KEY>`. If the key does not exist, the command SHALL succeed silently (idempotent). The command SHALL accept `--session <name>` and `--repo <name|path>` with the same defaults as `set`.

#### Scenario: Unset existing env var
- **WHEN** user runs `ez session env unset AWS_PROFILE --session feature-x`
- **THEN** system removes `AWS_PROFILE` from the session's env map and persists
- **THEN** system prints confirmation: `Unset AWS_PROFILE on session feature-x`

#### Scenario: Unset non-existent key
- **WHEN** user runs `ez session env unset NONEXISTENT --session feature-x`
- **THEN** system succeeds silently (no error, no output)

#### Scenario: Auto-detect current session for unset
- **WHEN** user runs `ez session env unset MY_VAR` without `--session` while inside a registered session
- **THEN** system detects the current session and removes the var from it
