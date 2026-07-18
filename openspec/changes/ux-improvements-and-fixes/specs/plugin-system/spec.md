# Plugin System (Delta)

## ADDED Requirements

### Requirement: GitHub PR branch resolution hook

The system SHALL support an `OnPRResolve` hook type that plugins can declare to resolve a GitHub PR URL into a branch name. The hook request SHALL include the PR URL. The hook response SHALL include an optional `branch_name` field. This hook is invoked during the "From GitHub PR" name builder mode to determine which branch the worktree should track.

#### Scenario: Plugin resolves PR to branch

- **WHEN** the "From GitHub PR" name builder mode provides a PR URL and a plugin declares `OnPRResolve`
- **THEN** system invokes the hook with the PR URL in the request
- **THEN** if the plugin responds with `branch_name`, the worktree is created tracking that branch

#### Scenario: Plugin returns no branch

- **WHEN** the plugin responds to `OnPRResolve` without a `branch_name`
- **THEN** system proceeds with default behavior (create a new branch named `pr<number>`)

#### Scenario: No plugin handles OnPRResolve

- **WHEN** no enabled plugin declares the `OnPRResolve` hook
- **THEN** system skips the hook invocation and proceeds with default worktree creation

#### Scenario: Hook request format

- **WHEN** `OnPRResolve` is invoked
- **THEN** the `HookRequest` includes `hook: "OnPRResolve"` and a `pr_context` field with `url` (the full PR URL), `number` (the extracted PR number), and `repo` (the repo entry)
