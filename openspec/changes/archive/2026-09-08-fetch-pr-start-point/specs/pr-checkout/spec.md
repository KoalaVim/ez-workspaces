## MODIFIED Requirements

### Requirement: Start point override for PR branch
When the PR branch exists on the remote, the git-worktree plugin SHALL use the remote branch as the start point for the worktree creation. The session's `start_point` SHALL be set to `origin/<headRefName>` to ensure the worktree has the full PR branch history. This applies identically whether the PR was identified by URL or by number.

Before using an override start point of the form `origin/<ref>`, the plugin SHALL fetch `<ref>` from `origin`, so that a PR branch the local clone has never fetched resolves. This fetch SHALL NOT be suppressed by the fetch-TTL cache, which governs only the periodic refresh of the default start point. The override SHALL be honored on every session-create route, including the route where another `mutates_session_path` plugin has already set the session path. When the override ref still cannot be resolved after the fetch, session creation SHALL fail with an error naming the unresolved start point rather than silently falling back to another start point.

#### Scenario: Remote PR branch used as start point
- **WHEN** PR checkout creates a session and the PR branch exists on origin
- **THEN** the git-worktree plugin creates the worktree with start point `origin/<headRefName>`

#### Scenario: Start point from number-resolved PR
- **WHEN** the PR was identified by a bare number and resolved via `gh`
- **THEN** `ez_start_point` is set to `origin/<headRefName>` just as for a URL-resolved PR

#### Scenario: PR branch not yet fetched locally
- **WHEN** PR checkout creates a session whose start point is `origin/<headRefName>` and the local clone has no such remote-tracking ref
- **THEN** the plugin fetches `<headRefName>` from `origin` before creating the worktree
- **THEN** the worktree is created at the PR branch head

#### Scenario: Fetch cache does not skip the override fetch
- **WHEN** a PR session is created within the fetch TTL of a previous fetch
- **THEN** the override ref is still fetched from `origin`

#### Scenario: Override honored when session path is pre-set
- **WHEN** another `mutates_session_path` plugin has already set the session path and `ez_start_point` is set
- **THEN** the branch is created from that start point rather than from the resolved default start point

#### Scenario: Override ref unresolvable after fetch
- **WHEN** the override start point cannot be resolved after fetching from `origin` (for example a PR opened from a fork)
- **THEN** session creation reports an error that names the unresolved start point
