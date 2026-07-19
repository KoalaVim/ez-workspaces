# Repo Management (Delta)

## MODIFIED Requirements

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
