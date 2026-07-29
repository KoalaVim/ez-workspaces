# PR Checkout

## Purpose

Enable creating sessions from GitHub PR URLs with full branch resolution and remote tracking.

## Requirements

### Requirement: PR branch resolution via gh CLI
The "From GitHub PR" name builder mode SHALL use `gh pr view <url> --json headRefName,baseRefName,number` to resolve the PR's actual branch name and base branch. The session name SHALL be set to the PR's branch name (not `pr<number>`). The PR number and URL SHALL be stored in `session.env` as `ez_pr_number` and `ez_pr_url`.

#### Scenario: Resolve PR branch from URL
- **WHEN** user pastes a GitHub PR URL in the "From GitHub PR" name builder mode
- **THEN** system runs `gh pr view <url> --json headRefName,baseRefName,number`
- **THEN** session name is set to the PR's `headRefName` (branch name)
- **THEN** `ez_pr_number` and `ez_pr_url` are stored in `session.env`

#### Scenario: gh CLI not available
- **WHEN** `gh` is not installed or not in PATH
- **THEN** system falls back to the current behavior (extract `pr<number>` from URL) and warns the user

#### Scenario: gh auth failure
- **WHEN** `gh pr view` fails due to authentication
- **THEN** system falls back to extracting `pr<number>` from URL and displays the `gh` error

### Requirement: Start point override for PR branch
When the PR branch exists on the remote, the git-worktree plugin SHALL use the remote branch as the start point for the worktree creation. The session's `start_point` SHALL be set to `origin/<headRefName>` to ensure the worktree has the full PR branch history.

#### Scenario: Remote PR branch used as start point
- **WHEN** PR checkout creates a session and the PR branch exists on origin
- **THEN** the git-worktree plugin creates the worktree with start point `origin/<headRefName>`
