## MODIFIED Requirements

### Requirement: PR branch resolution via gh CLI
The "From GitHub PR" name builder mode SHALL use `gh pr view <ref> --json headRefName,number,url` to resolve the PR's actual branch name, number, and canonical URL, where `<ref>` is either the pasted PR URL or the entered PR number. The command SHALL run with the repo root as its working directory when repo context is available, so that a bare PR number resolves against the repo's own GitHub remote. The session name SHALL be set to the PR's branch name (not `pr<number>`). The PR number and URL SHALL be stored in `session.env` as `ez_pr_number` and `ez_pr_url`; when the input was a bare number, `ez_pr_url` SHALL be the `url` reported by `gh`.

#### Scenario: Resolve PR branch from URL
- **WHEN** user pastes a GitHub PR URL in the "From GitHub PR" name builder mode
- **THEN** system runs `gh pr view <url> --json headRefName,number,url`
- **THEN** session name is set to the PR's `headRefName` (branch name)
- **THEN** `ez_pr_number` and `ez_pr_url` are stored in `session.env`

#### Scenario: Resolve PR branch from number
- **WHEN** user enters a bare PR number in the "From GitHub PR" name builder mode from within a repo
- **THEN** system runs `gh pr view <number> --json headRefName,number,url` with the repo root as the working directory
- **THEN** session name is set to the PR's `headRefName`
- **THEN** `ez_pr_number` is the resolved number and `ez_pr_url` is the canonical URL returned by `gh`

#### Scenario: gh CLI not available
- **WHEN** `gh` is not installed or not in PATH
- **THEN** system falls back to the current behavior (extract `pr<number>` from the input) and warns the user

#### Scenario: gh auth failure
- **WHEN** `gh pr view` fails due to authentication
- **THEN** system falls back to extracting `pr<number>` from the input and displays the `gh` error

#### Scenario: PR number not found in repo
- **WHEN** the entered number does not correspond to a PR in the repo's GitHub remote
- **THEN** system displays the `gh` error and falls back to `pr<number>` as the session name

### Requirement: Start point override for PR branch
When the PR branch exists on the remote, the git-worktree plugin SHALL use the remote branch as the start point for the worktree creation. The session's `start_point` SHALL be set to `origin/<headRefName>` to ensure the worktree has the full PR branch history. This applies identically whether the PR was identified by URL or by number.

#### Scenario: Remote PR branch used as start point
- **WHEN** PR checkout creates a session and the PR branch exists on origin
- **THEN** the git-worktree plugin creates the worktree with start point `origin/<headRefName>`

#### Scenario: Start point from number-resolved PR
- **WHEN** the PR was identified by a bare number and resolved via `gh`
- **THEN** `ez_start_point` is set to `origin/<headRefName>` just as for a URL-resolved PR
