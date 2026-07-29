## Context

The PR checkout flow currently creates a session from a GitHub PR URL, resolves the branch via `gh`, creates a worktree, then resets to the merge-base to show PR changes as dirty files. This was intended for a review-focused workflow but creates friction for the more common use case of working directly on the PR branch.

The `pr_merge_base_reset` function in `src/browser/mod.rs` is called from two locations:
1. `src/session/mod.rs` (`new_session`) — after CLI-driven session creation
2. `src/browser/mod.rs` — after browser-driven session creation

## Goals / Non-Goals

**Goals:**
- PR checkout creates a normal worktree with the PR branch checked out and full commit history intact
- Remove all merge-base reset logic and related code paths

**Non-Goals:**
- Changing the PR branch resolution or `gh` integration (that stays as-is)
- Adding an opt-in flag to restore the old reset behavior (can be added later if requested)
- Changing the `start_point` override logic for PR branches

## Decisions

1. **Full removal over opt-in toggle**: Remove `pr_merge_base_reset` entirely rather than hiding it behind a config flag. The function is small and can be reintroduced if needed. Simpler code wins.

2. **Keep `PrMetadata.base_ref` field**: The `base_ref` field on `PrMetadata` is still stored in session env and used by the preview/indicator logic. Only the reset call site is removed.

3. **Keep the spec requirement for "Start point override"**: The `origin/<headRefName>` start-point logic in the git-worktree plugin remains unchanged — this ensures the full PR branch is available in the worktree.

## Risks / Trade-offs

- [Users who relied on dirty-file view] → They can run the reset manually. The command is documented in `pr-checkout` spec under migration.
- [Spec drift] → The main `openspec/specs/pr-checkout/spec.md` must be updated (via delta archive) to remove the reset requirement and related scenarios.
