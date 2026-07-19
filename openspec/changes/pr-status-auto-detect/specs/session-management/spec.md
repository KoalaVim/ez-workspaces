## MODIFIED Requirements

### Requirement: Enter session
The system SHALL enter a session by applying the `on_enter` action. The default action is `cd` (write the session's worktree path to the cd-file). The action can be overridden to a plugin-bind name (e.g. `tmux`), which runs that bind's `OnBind` hook. If the plugin bind produces no navigation effect, the system SHALL fall back to `cd`.

Before applying the enter action, if the session is git-backed, not bare, not the default session, has no `ez_pr_number` in its env, and `gh` is available, the system SHALL attempt to auto-detect a GitHub PR associated with the session's branch using `gh pr list --head <branch> --json number,url,state --limit 1`. If a PR is found, the system SHALL populate `ez_pr_number`, `ez_pr_url`, and `ez_pr_status` in the session env and persist the change. Detection is best-effort and SHALL be silently skipped if `gh` is not installed, not authenticated, or the command fails.

If the session already has `ez_pr_number` set, the existing refresh logic (re-check if status is older than 5 minutes) SHALL apply instead.

#### Scenario: Default cd enter
- **WHEN** user enters a session with `on_enter = "cd"`
- **THEN** system writes the session's path to the cd-file for the shell wrapper to cd into

#### Scenario: Plugin bind enter
- **WHEN** user enters a session with `on_enter = "tmux"`
- **THEN** system finds the matching session-context plugin bind and runs its `OnBind` hook
- **THEN** if the hook returns `cd_target` or `post_shell_commands`, those are applied

#### Scenario: Plugin bind fallback
- **WHEN** the plugin bind produces no navigation effect or fails
- **THEN** system falls back to plain `cd` into the session path

#### Scenario: Auto-detect PR on session enter
- **WHEN** user enters a git-backed, non-bare, non-default session that has no `ez_pr_number` env
- **AND** `gh` CLI is installed and the session's branch has an open PR on GitHub
- **THEN** the system populates `ez_pr_number`, `ez_pr_url`, `ez_pr_status` in the session env
- **AND** persists the updated session to disk
- **AND** the PR indicator appears in the session picker on subsequent renders

#### Scenario: Auto-detect PR with no PR found
- **WHEN** user enters a session whose branch has no associated PR
- **THEN** no env vars are set and the session is unchanged
- **AND** the next enter will attempt detection again

#### Scenario: Auto-detect PR without gh
- **WHEN** `gh` CLI is not installed or not authenticated
- **THEN** detection is silently skipped with a debug log
