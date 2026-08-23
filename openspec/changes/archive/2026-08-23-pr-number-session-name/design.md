## Context

When a session is created via the "From GitHub PR" name builder mode, `resolve_pr_via_gh` in `src/session/name_builder.rs` resolves the PR's branch name via `gh pr view` and uses it as the session name. The branch name is also used as the worktree's checkout branch (via `ez_start_point` in the session env).

Currently the session name is set to the raw `headRefName` (e.g. `feature/add-widget`), making PR-sourced sessions visually indistinguishable from manually created ones.

## Goals / Non-Goals

**Goals:**
- Session name for PR-created sessions becomes `pr<number>-<branch-name>` (e.g. `pr42-feature/add-widget`)
- The worktree still checks out the actual PR branch — only the session name changes

**Non-Goals:**
- Changing the fallback name when `gh` fails (stays `pr<number>`)
- Changing how `ez_start_point`, `ez_pr_number`, or `ez_pr_url` env vars are populated
- Changing session naming for non-PR modes (full name, build from parts, Jira)

## Decisions

**Format the session name in `resolve_pr_via_gh`**: The name is composed at a single return site in `resolve_pr_via_gh` (line 372). Change `(head_ref, Some(metadata))` to `(format!("pr{number}-{head_ref}"), Some(metadata))`. This keeps the naming logic co-located with the PR resolution logic rather than pushing it to callers.

Alternative considered: formatting the name in the callers (`prompt_github_pr`, `new_session`, `create_child_session`). Rejected because the name is already fully formed when returned from `resolve_pr_via_gh`, and all callers use it identically.

## Risks / Trade-offs

- [Session name collisions] Sessions named `pr42-feature-branch` could collide with a manually created session of the same name → Same risk as today with branch-name collisions; the existing `SessionAlreadyExists` guard handles it.
- [Longer session names] The `pr<num>-` prefix adds ~5 characters → Acceptable trade-off for clarity. Session names are already unbounded in length (branch names can be long).
