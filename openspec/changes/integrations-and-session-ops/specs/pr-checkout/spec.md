# PR Checkout (Delta)

## ADDED Requirements

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

### Requirement: Reset to merge-base after checkout
After the worktree is created with the PR's branch, the system SHALL run `git reset --mixed $(git merge-base HEAD origin/<base-branch>)` in the new worktree to unstage all PR commits. This presents the PR's changes as dirty/unstaged files in the worktree.

#### Scenario: PR changes shown as dirty
- **WHEN** session creation completes for a PR checkout
- **THEN** system runs `git reset --mixed` to the merge-base in the new worktree
- **THEN** all files changed by the PR appear as unstaged modifications

#### Scenario: Merge-base resolution
- **WHEN** system computes the merge-base
- **THEN** it uses `git merge-base HEAD origin/<baseRefName>` where `baseRefName` is from the `gh pr view` output

#### Scenario: Reset failure
- **WHEN** `git reset` fails (e.g. merge-base not found)
- **THEN** system warns the user but keeps the session (worktree has the PR branch checked out normally)

### Requirement: Start point override for PR branch
When the PR branch exists on the remote, the git-worktree plugin SHALL use the remote branch as the start point for the worktree creation. The session's `start_point` SHALL be set to `origin/<headRefName>` to ensure the worktree has the full PR branch history.

#### Scenario: Remote PR branch used as start point
- **WHEN** PR checkout creates a session and the PR branch exists on origin
- **THEN** the git-worktree plugin creates the worktree with start point `origin/<headRefName>`
