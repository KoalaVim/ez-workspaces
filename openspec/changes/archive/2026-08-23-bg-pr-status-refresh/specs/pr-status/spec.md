## MODIFIED Requirements

### Requirement: PR status storage in session env
Sessions associated with a GitHub PR SHALL store PR metadata in `session.env`: `ez_pr_number` (PR number), `ez_pr_url` (full URL), `ez_pr_status` (one of `open`, `merged`, `closed`), and `ez_pr_gh_user` (GitHub username used to fetch the data). The status SHALL be refreshed by the background daemon process rather than synchronously on session enter.

#### Scenario: PR status set on PR checkout
- **WHEN** a session is created via the PR checkout workflow
- **THEN** `ez_pr_status` is set to the PR's current state (e.g. `open`)
- **AND** `ez_pr_gh_user` is set to the currently authenticated GitHub username

#### Scenario: PR status refreshed by daemon
- **WHEN** the background daemon runs a polling cycle
- **THEN** it refreshes `ez_pr_status` for all sessions with stale data and a matching `ez_pr_gh_user`

#### Scenario: PR status NOT refreshed synchronously on enter
- **WHEN** user enters a session that has `ez_pr_number` set
- **THEN** the system does NOT call `gh pr view` synchronously — the daemon handles refresh

#### Scenario: gh CLI not available for refresh
- **WHEN** `gh` is not installed during a daemon refresh attempt
- **THEN** system keeps the existing `ez_pr_status` value (no error, just stale data)

### Requirement: PR status auto-detection
When entering a session that is git-backed, not bare, not the default session, and has no `ez_pr_number` in its env, the system SHALL attempt to auto-detect a GitHub PR using `gh pr list --head <branch> --state all --json number,url,state --limit 1`. The command SHALL be run from the repo's root directory. If a PR is found, the system SHALL populate `ez_pr_number`, `ez_pr_url`, `ez_pr_status`, and `ez_pr_gh_user`. Detection also runs when a session is selected in the interactive browser.

#### Scenario: Auto-detect on enter
- **WHEN** user enters a session with no PR metadata and `gh` is available
- **THEN** system queries `gh pr list --head <branch> --state all` from the repo directory
- **THEN** if a PR is found, metadata including `ez_pr_gh_user` is populated and persisted

#### Scenario: Auto-detect in browser
- **WHEN** user selects a session in the interactive browser
- **THEN** system triggers auto-detection before updating `last_accessed`

#### Scenario: Merged/closed PR detected
- **WHEN** the branch has a merged or closed PR
- **THEN** detection finds it via `--state all` and populates the status accordingly

#### Scenario: No PR found
- **WHEN** the branch has no associated PR
- **THEN** no env vars are set and detection will retry on next enter
