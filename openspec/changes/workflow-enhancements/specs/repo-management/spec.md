# Repo Management (Delta)

## MODIFIED Requirements

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

### Requirement: Repo identity
Each repo SHALL have a unique `id` (slug derived from path), a `name` (directory name), an absolute `path`, a `registered_at` timestamp, and an `is_git` boolean flag (default `true`). Per-repo metadata SHALL include optional `remote_url`, `default_branch`, labels, and plugin state. For non-git repos, `remote_url` and `default_branch` SHALL be `None`.

#### Scenario: Slug derivation
- **WHEN** a repo at `~/workspace/personal/my-repo` is registered
- **THEN** the repo gets an id like `personal-my-repo` derived from the workspace-relative path

#### Scenario: Non-git repo identity
- **WHEN** a non-git directory at `~/workspace/notes` is registered
- **THEN** the repo gets `is_git = false`, `remote_url = None`, `default_branch = None`

## ADDED Requirements

### Requirement: Last-accessed timestamp on repo metadata
The `RepoMeta` SHALL include a `last_accessed` timestamp field that records when the repo was last browsed in the interactive browser. This field SHALL be updated when the user enters the session picker for a repo. It SHALL default to `registered_at` for repos that have never been browsed.

#### Scenario: Timestamp initialized on registration
- **WHEN** a new repo is registered
- **THEN** `last_accessed` is set to `registered_at`

#### Scenario: Timestamp updated on browse
- **WHEN** user selects a repo in the browser and enters its session picker
- **THEN** `last_accessed` is updated to the current time and persisted
