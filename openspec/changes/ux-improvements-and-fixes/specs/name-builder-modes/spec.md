# Name Builder Modes

## Purpose

Provide configurable modes for the session name builder, allowing users to choose how they want to construct session names — from a simple free-text entry to structured extraction from GitHub PRs or Jira URLs. A mode selection step is presented before the staged builder begins.

## ADDED Requirements

### Requirement: Mode selection step

The system SHALL present a mode selection prompt before entering the staged name builder when creating a session interactively. The available modes SHALL be configurable. The user selects a mode via fzf, and the system dispatches to the corresponding mode handler.

#### Scenario: Mode picker displayed

- **WHEN** user creates a session interactively (no name provided or `--interactive` flag)
- **THEN** system presents a mode selection list with all configured modes before any name building begins

#### Scenario: Single mode configured

- **WHEN** only one mode is configured in `name_builder_modes`
- **THEN** system skips the mode picker and enters that mode directly

#### Scenario: Cancel mode selection

- **WHEN** user presses Escape at the mode selection step
- **THEN** session creation is cancelled and the system returns to the previous context

### Requirement: Full name mode

The "Full name" mode SHALL skip all stages and present a single free-text prompt where the user types the entire session name directly.

#### Scenario: Type full name

- **WHEN** user selects "Full name" mode
- **THEN** system prompts with a single free-text input for the complete session name
- **THEN** the entered text becomes the session name without any prefix/suffix processing

#### Scenario: Empty input rejected

- **WHEN** user submits an empty string in "Full name" mode
- **THEN** system rejects the input and re-prompts

### Requirement: Build from parts mode

The "Build from parts" mode SHALL use the existing multi-stage session name builder behavior with configured stages (prefix, ticket-prefix, ticket-number, description).

#### Scenario: Staged builder flow

- **WHEN** user selects "Build from parts" mode
- **THEN** system enters the configured multi-stage name builder (same as existing behavior)
- **THEN** parts are joined with `-` and `(none)` stages contribute nothing

### Requirement: From GitHub PR mode

The "From GitHub PR" mode SHALL prompt the user to paste a GitHub PR URL, extract the PR number as `pr<number>`, and invoke a plugin hook to optionally resolve the PR's branch name for the worktree. If the plugin hook is available and returns a branch name, the system SHALL use that branch for the worktree instead of creating a new one.

#### Scenario: Paste PR URL and extract number

- **WHEN** user selects "From GitHub PR" mode and pastes `https://github.com/org/repo/pull/42`
- **THEN** system extracts the PR number and sets the session name to `pr42`

#### Scenario: Plugin resolves branch

- **WHEN** the `OnPRResolve` plugin hook is available and the PR URL is provided
- **THEN** system invokes the hook with the PR URL
- **THEN** if the hook returns a branch name, the worktree is created on that branch instead of a new one

#### Scenario: Plugin hook unavailable

- **WHEN** no plugin handles `OnPRResolve`
- **THEN** system proceeds with `pr<number>` as the session name and creates a standard worktree

#### Scenario: Invalid PR URL

- **WHEN** user pastes a string that is not a valid GitHub PR URL
- **THEN** system shows an error and re-prompts

### Requirement: From Jira URL mode

The "From Jira URL" mode SHALL prompt the user to paste a Jira issue URL, extract the project key and issue number (e.g. `PROJ-123`), and then continue with the staged builder for an optional descriptive suffix.

#### Scenario: Paste Jira URL and extract ticket

- **WHEN** user selects "From Jira URL" mode and pastes `https://company.atlassian.net/browse/PROJ-123`
- **THEN** system extracts `PROJ-123` as the ticket prefix
- **THEN** system prompts for an optional descriptive suffix via free-text input
- **THEN** resulting name is `PROJ-123-<suffix>` (or just `PROJ-123` if no suffix provided)

#### Scenario: Invalid Jira URL

- **WHEN** user pastes a string that is not a valid Jira issue URL
- **THEN** system shows an error and re-prompts

### Requirement: Configurable available modes

The system SHALL allow users to configure which modes are available in the mode picker via the `name_builder_modes` config field. The default set SHALL include all modes: "Full name", "Build from parts", "From GitHub PR", "From Jira URL".

#### Scenario: Restrict to subset

- **WHEN** config has `name_builder_modes = ["full_name", "build_from_parts"]`
- **THEN** the mode picker only shows "Full name" and "Build from parts"

#### Scenario: Default modes

- **WHEN** no `name_builder_modes` config is specified
- **THEN** all four modes are available in the mode picker
