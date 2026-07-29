## Why

When creating a session from a GitHub PR URL, the current behavior automatically resets the worktree to the merge-base (`git reset --mixed $(git merge-base HEAD origin/<base>)`), presenting PR changes as dirty/unstaged files. This was designed for code-review workflows, but in practice most users want to work on the PR branch normally — with the full commit history intact and no artificial dirty state. The reset makes standard git operations (commit, push, rebase) confusing and forces users to undo it manually.

## What Changes

- Remove the automatic `git reset --mixed` to merge-base after PR checkout. The worktree will retain the PR branch as-is with its full commit history.
- Remove the "PR changes shown as dirty files" feedback message.
- The `pr_merge_base_reset` function and its call sites will be removed entirely.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `pr-checkout`: Remove the "Reset to merge-base after checkout" requirement. PR sessions will simply check out the branch without resetting.

## Impact

- `src/browser/mod.rs`: Remove `pr_merge_base_reset` function and its call site in the browser session-create flow.
- `src/session/mod.rs`: Remove the `pr_merge_base_reset` call after session creation in `new_session`.
- `openspec/specs/pr-checkout/spec.md`: Remove the "Reset to merge-base after checkout" requirement and related scenarios.
