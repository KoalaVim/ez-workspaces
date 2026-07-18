# Session Management (Delta)

## MODIFIED Requirements

### Requirement: Create session

The system SHALL create a new session with a unique UUID, user-facing name, and timestamp. If a parent session ID is provided, the new session SHALL be a child of that parent. If no parent is specified, the system SHALL default `parent_id` to the repo's default (main) session, making the new session a child of `main`. On creation, the system SHALL run `OnSessionCreate` plugin hooks in `mutates_session_path`-first order so downstream plugins see the resolved path. After worktree creation, the system SHALL fetch the target branch from the remote (in addition to the base branch) to ensure the local ref is up to date.

#### Scenario: Create root-level session

- **WHEN** user runs `ez session new feature-auth`
- **THEN** system creates a session named `feature-auth` with `parent_id` set to the default (main) session's UUID, runs `OnSessionCreate` hooks, and saves to `sessions.toml`

#### Scenario: Create child session

- **WHEN** user runs `ez session new sub-task --parent feature-auth`
- **THEN** system creates a session named `sub-task` with `parent_id` set to `feature-auth`'s UUID

#### Scenario: Duplicate name rejected

- **WHEN** user creates a session with a name that already exists in the same repo
- **THEN** system returns `SessionAlreadyExists` error

#### Scenario: Branch reuse prompt

- **WHEN** the session name matches an existing git branch in the repo
- **THEN** system prompts the user to reuse the existing branch or recreate from the latest base

#### Scenario: Enhanced branch fetch on create

- **WHEN** a session is created and the target branch exists on the remote
- **THEN** system fetches the target branch from origin (e.g. `git fetch origin <branch>`) in addition to the base branch fetch
- **THEN** the worktree is created with the latest remote state of that branch

### Requirement: Multi-stage session name builder

The system SHALL support configurable name stages when creating a session interactively without a pre-supplied name. Before entering the stages, the system SHALL present a mode selection step allowing the user to choose how to build the name (see name-builder-modes capability). If "Build from parts" mode is selected, each stage is either a `choice` (fzf list with `(custom)` and `(none)` sentinels) or `text` (free-text input). After all configured stages, a final free-text prompt is always shown. Parts are joined with `-`; `(none)` stages contribute nothing. Each stage supports Ctrl-P to go back to the previous stage.

#### Scenario: Default stages

- **WHEN** user creates a session interactively without a name and selects "Build from parts" mode
- **THEN** system prompts through stages: prefix (feat/fix/chore), ticket-prefix (custom), ticket-number (text), then final description
- **THEN** resulting name is the non-empty parts joined by `-` (e.g. `feat-ABC-123-add-dark-mode`)

#### Scenario: Skip stages

- **WHEN** user selects `(none)` for a stage
- **THEN** that stage contributes nothing to the name

#### Scenario: Back navigation

- **WHEN** user presses Ctrl-P during a stage
- **THEN** system returns to the previous stage with prior context preserved

#### Scenario: Name provided on CLI

- **WHEN** user passes a name argument (e.g. `ez session new my-feature`) without `--interactive`
- **THEN** the stage builder is skipped entirely

#### Scenario: Mode selection before stages

- **WHEN** user creates a session interactively without a name
- **THEN** system presents the mode selection step first, then dispatches to the chosen mode handler

### Requirement: Register existing worktree

The system SHALL register an existing git worktree as a session without running `OnSessionCreate` hooks. It resolves the worktree root and common repo via `git rev-parse`, matches that repo to the registered repo index, and writes a `Session` with `path` set to the existing worktree. If no parent is specified, the system SHALL default `parent_id` to the repo's default (main) session.

#### Scenario: Register from current directory

- **WHEN** user runs `ez session register` inside a worktree
- **THEN** system detects the worktree root, matches the repo, and creates a session with the current branch name as a child of the default (main) session

#### Scenario: Register with explicit name and parent

- **WHEN** user runs `ez session register --name my-session --parent main`
- **THEN** system creates a session with the given name as a child of `main`

#### Scenario: Register defaults to main parent

- **WHEN** user runs `ez session register --name my-session` without `--parent`
- **THEN** system creates the session as a child of the default (main) session

## ADDED Requirements

### Requirement: Interactive builder flag

The system SHALL support a `--interactive` / `-i` flag on `ez session new` that forces the multi-stage name builder (starting from mode selection) even when a name is provided on the CLI. This allows users to use the interactive builder while passing other flags like `--parent`.

#### Scenario: Force interactive with name provided

- **WHEN** user runs `ez session new my-feature --interactive`
- **THEN** system ignores the provided name and enters the interactive mode selection and name builder flow

#### Scenario: Force interactive without name

- **WHEN** user runs `ez session new -i`
- **THEN** system enters the interactive mode selection and name builder flow (same as omitting the name)

#### Scenario: Short flag

- **WHEN** user runs `ez session new -i --parent feature-auth`
- **THEN** system enters interactive builder flow and creates the session as a child of `feature-auth`
