# Repo Management

## Purpose

Manage the lifecycle of registered git repositories. Repos are tracked in a global index (`~/.config/ez/repos/index.toml`) with per-repo metadata stored alongside sessions. The repo module handles clone, add, remove, list, and label operations, with plugin hooks fired at key lifecycle points.

## Requirements

### Requirement: Clone and register repo
The system SHALL clone a git repository from a URL, register it in the global index, and run `OnRepoClone` plugin hooks.

#### Scenario: Clone with default path
- **WHEN** user runs `ez clone https://github.com/user/repo.git`
- **THEN** system clones the repo to the current directory, registers it with a slug derived from the path, and runs `OnRepoClone` hooks

#### Scenario: Clone with explicit path
- **WHEN** user runs `ez clone https://github.com/user/repo.git ~/workspace/my-repo`
- **THEN** system clones to the specified path and registers it

### Requirement: Add existing repo
The system SHALL register an existing git repository in the global index without cloning. It SHALL detect the repo root from the provided path or current directory.

#### Scenario: Add from current directory
- **WHEN** user runs `ez add` inside a git repo
- **THEN** system registers the current directory's git root

#### Scenario: Duplicate registration rejected
- **WHEN** user adds a repo that is already registered
- **THEN** system returns `RepoAlreadyRegistered` error

### Requirement: Remove repo
The system SHALL unregister a repository by name or ID. With `--purge`, it SHALL also delete all session metadata and plugin state for that repo. The system SHALL run `OnRepoRemove` plugin hooks.

#### Scenario: Unregister repo
- **WHEN** user runs `ez repo remove my-repo`
- **THEN** system removes the repo from the index and runs `OnRepoRemove` hooks

#### Scenario: Purge repo data
- **WHEN** user runs `ez repo remove my-repo --purge`
- **THEN** system removes the repo from the index AND deletes `~/.config/ez/repos/<id>/` directory

### Requirement: List repos
The system SHALL list all registered repositories with their names, paths, and current branches. The list SHALL support filtering by label.

#### Scenario: List all repos
- **WHEN** user runs `ez repo list`
- **THEN** system displays all registered repos with name, path, and branch

#### Scenario: Filter by label
- **WHEN** user runs `ez repo list --label backend`
- **THEN** system displays only repos that carry the `backend` label

### Requirement: Repo labels
Repos SHALL support arbitrary string labels for grouping and filtering. Labels are stored in per-repo metadata (`repo.toml`).

#### Scenario: Add labels
- **WHEN** user runs `ez repo label add my-repo backend core`
- **THEN** system adds labels `backend` and `core` to the repo

#### Scenario: Remove labels
- **WHEN** user runs `ez repo label remove my-repo core`
- **THEN** system removes label `core` from the repo

#### Scenario: List all labels grouped
- **WHEN** user runs `ez repo label list` without a target
- **THEN** system lists all labels across all repos, grouping repos under each label

### Requirement: Repo identity
Each repo SHALL have a unique `id` (slug derived from path), a `name` (directory name), an absolute `path`, and a `registered_at` timestamp. Per-repo metadata SHALL include optional `remote_url`, `default_branch`, labels, and plugin state.

#### Scenario: Slug derivation
- **WHEN** a repo at `~/workspace/personal/my-repo` is registered
- **THEN** the repo gets an id like `personal-my-repo` derived from the workspace-relative path

### Requirement: Owner parsing
The system SHALL parse the "owner" portion from git remote URLs to support grouping repos by owner. It SHALL support HTTPS, SSH shorthand (`git@host:OWNER/repo`), SSH scheme, and git scheme URLs.

#### Scenario: Parse HTTPS URL
- **WHEN** remote URL is `https://github.com/rust-lang/rust.git`
- **THEN** owner is parsed as `rust-lang`

#### Scenario: Parse SSH shorthand
- **WHEN** remote URL is `git@github.com:ofirg/ez-workspaces.git`
- **THEN** owner is parsed as `ofirg`

#### Scenario: Invalid URL returns None
- **WHEN** remote URL is empty or malformed
- **THEN** owner parsing returns `None`

### Requirement: Auto-register on browse
The system SHALL auto-register a repo when the user navigates to it in the interactive browser and it is not yet registered.

#### Scenario: Auto-register during drill-down
- **WHEN** user drills into a git repo directory in the workspace browser
- **THEN** system auto-registers the repo and proceeds to the session picker
