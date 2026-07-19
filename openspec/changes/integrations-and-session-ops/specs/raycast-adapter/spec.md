# Raycast Adapter (Delta)

## ADDED Requirements

### Requirement: JSON output for repo list
The `ez repo list` command SHALL support a `--json` flag that outputs a JSON array of repo objects with fields: `id`, `name`, `path`, `is_git`, `default_branch`, `remote_url`, `labels`.

#### Scenario: JSON repo list
- **WHEN** user runs `ez repo list --json`
- **THEN** system outputs a JSON array of all registered repos with structured fields

#### Scenario: JSON repo list with label filter
- **WHEN** user runs `ez repo list --json --label backend`
- **THEN** system outputs a JSON array of repos matching the label filter

### Requirement: JSON output for session list
The `ez session list` command SHALL support a `--json` flag that outputs a JSON array of session objects with fields: `id`, `name`, `parent_id`, `path`, `bare`, `labels`, `last_accessed`, `env`, `is_default`.

#### Scenario: JSON session list
- **WHEN** user runs `ez session list --json --repo my-repo`
- **THEN** system outputs a JSON array of all sessions for the specified repo

### Requirement: Raycast repo launcher script
A Raycast script command SHALL list all registered repos and allow the user to select one to open. On selection, it SHALL either open a terminal and run `ez --repo <path>` or directly enter the repo's default session.

#### Scenario: List repos in Raycast
- **WHEN** user triggers the Raycast repo launcher
- **THEN** Raycast displays all registered repos with name and path
- **THEN** selecting a repo opens a terminal with `ez --repo <path>`

### Requirement: Raycast session launcher script
A Raycast script command SHALL list sessions for a selected repo and allow the user to enter one. It SHALL use `ez session enter` or the configured `on_enter` action.

#### Scenario: List sessions in Raycast
- **WHEN** user triggers the Raycast session launcher for a specific repo
- **THEN** Raycast displays all sessions with name and status indicators
- **THEN** selecting a session runs `ez session enter <name> --repo <id>`
