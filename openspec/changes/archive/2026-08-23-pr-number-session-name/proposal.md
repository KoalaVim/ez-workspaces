## Why

When creating a session from a GitHub PR, the session is named after the PR's branch (e.g. `feature/add-widget`). This makes it hard to distinguish PR-sourced sessions from manually created ones at a glance, and loses the PR number from the visible session name. Prefixing with `pr<num>-` makes the origin immediately clear and keeps the PR number visible without needing to inspect session env vars.

## What Changes

- Change the session name format for PR-created sessions from `<branch-name>` to `pr<num>-<branch-name>` (e.g. `pr42-feature/add-widget`)
- The worktree still checks out the PR's actual branch — only the session name changes
- The fallback name (when `gh` fails) remains `pr<num>` (unchanged)

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `pr-checkout`: The session naming requirement changes — from using `headRefName` alone to `pr<number>-<headRefName>`. The start point and env var behavior remain the same.

## Impact

- `src/session/name_builder.rs`: `resolve_pr_via_gh` returns the new name format
- Existing `pr-checkout` spec scenarios need updating to reflect the new naming
- No API changes, no breaking changes — this is a cosmetic naming change
