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
The system SHALL register an existing git repository OR a non-git directory in the global index without cloning. For git repos, it SHALL detect the repo root from the provided path or current directory. For non-git directories, it SHALL register the directory as-is with `is_git = false` in the `RepoEntry`.

#### Scenario: Add from current directory
- **WHEN** user runs `ez add` inside a git repo
- **THEN** system registers the current directory's git root with `is_git = true`

#### Scenario: Duplicate registration rejected
- **WHEN** user adds a repo that is already registered
- **THEN** system returns `RepoAlreadyRegistered` error

#### Scenario: Add non-git directory
- **WHEN** user runs `ez add` inside a directory without `.git`
- **THEN** system registers the current directory with `is_git = false`

#### Scenario: Add non-git directory by explicit path
- **WHEN** user runs `ez add /path/to/plain-dir` where the target has no `.git`
- **THEN** system registers the directory with `is_git = false`

### Requirement: Remove repo
The system SHALL unregister a repository by name, ID, or path. When a path is provided (contains `/` or `.`), the system SHALL resolve it to an absolute path and find the matching `RepoEntry`. With `--purge`, it SHALL also delete all session metadata and plugin state for that repo. The system SHALL run `OnRepoRemove` plugin hooks. A top-level `ez remove <path>` alias SHALL be available.

#### Scenario: Unregister repo
- **WHEN** user runs `ez repo remove my-repo`
- **THEN** system removes the repo from the index and runs `OnRepoRemove` hooks

#### Scenario: Purge repo data
- **WHEN** user runs `ez repo remove my-repo --purge`
- **THEN** system removes the repo from the index AND deletes `~/.config/ez/repos/<id>/` directory

#### Scenario: Remove by path
- **WHEN** user runs `ez repo remove /Users/me/workspace/my-repo`
- **THEN** system resolves the path to the matching registered repo and removes it

#### Scenario: Remove by relative path
- **WHEN** user runs `ez repo remove .` inside a registered repo
- **THEN** system resolves the current directory to the matching repo and removes it

#### Scenario: Remove non-git directory by path
- **WHEN** user runs `ez remove ~/notes` where `~/notes` is a registered non-git repo
- **THEN** system resolves the path and removes the repo entry

#### Scenario: Top-level remove alias
- **WHEN** user runs `ez remove my-repo`
- **THEN** system delegates to `ez repo remove my-repo`

#### Scenario: Path not found
- **WHEN** user runs `ez repo remove /nonexistent/path`
- **THEN** system returns a "repo not found" error

### Requirement: List repos
The system SHALL list all registered repositories with their names, paths, and current branches. The list SHALL support filtering by label. The list SHALL support a `--json` flag that outputs a JSON array of repo objects with fields: `id`, `name`, `path`, `is_git`, `default_branch`, `remote_url`, `labels`.

#### Scenario: List all repos
- **WHEN** user runs `ez repo list`
- **THEN** system displays all registered repos with name, path, and branch

#### Scenario: Filter by label
- **WHEN** user runs `ez repo list --label backend`
- **THEN** system displays only repos that carry the `backend` label

#### Scenario: JSON repo list
- **WHEN** user runs `ez repo list --json`
- **THEN** system outputs a JSON array of all registered repos with structured fields

#### Scenario: JSON repo list with label filter
- **WHEN** user runs `ez repo list --json --label backend`
- **THEN** system outputs a JSON array of repos matching the label filter

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
Each repo SHALL have a unique `id` (slug derived from path), a `name` (directory name), an absolute `path`, a `registered_at` timestamp, and an `is_git` boolean flag (default `true`). Per-repo metadata SHALL include optional `remote_url`, `default_branch`, labels, and plugin state. For non-git repos, `remote_url` and `default_branch` SHALL be `None`.

#### Scenario: Slug derivation
- **WHEN** a repo at `~/workspace/personal/my-repo` is registered
- **THEN** the repo gets an id like `personal-my-repo` derived from the workspace-relative path

#### Scenario: Non-git repo identity
- **WHEN** a non-git directory at `~/workspace/notes` is registered
- **THEN** the repo gets `is_git = false`, `remote_url = None`, `default_branch = None`

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
The system SHALL auto-register a repo when the user navigates to it in the interactive browser and it is not yet registered. Before registering, the system SHALL check whether the path is already tracked as a session worktree under any registered repo. If a matching session is found, the system SHALL skip registration and use the owning repo entry instead.

#### Scenario: Auto-register during drill-down
- **WHEN** user drills into a git repo directory in the workspace browser
- **THEN** system auto-registers the repo and proceeds to the session picker

#### Scenario: Skip registration for session worktree
- **WHEN** user drills into a directory that is already tracked as a session worktree under a registered repo
- **THEN** system does NOT register it as a new repo
- **AND** system enters the session picker for the owning repo instead

### Requirement: Last-accessed timestamp on repo metadata
The `RepoMeta` SHALL include a `last_accessed` timestamp field that records when the repo was last browsed in the interactive browser. This field SHALL be updated when the user enters the session picker for a repo. It SHALL default to `registered_at` for repos that have never been browsed.

#### Scenario: Timestamp initialized on registration
- **WHEN** a new repo is registered
- **THEN** `last_accessed` is set to `registered_at`

#### Scenario: Timestamp updated on browse
- **WHEN** user selects a repo in the browser and enters its session picker
- **THEN** `last_accessed` is updated to the current time and persisted
