## Context

PR status tracking exists but is gated on `ez_pr_number` being present in the session env. That env var is only set during the GitHub PR name builder mode. Most sessions are created with manual names or the parts builder, so they never get PR metadata even when their branch has an open PR.

The `gh` CLI can discover PRs by branch name: `gh pr list --head <branch> --json number,url,state --limit 1`.

## Goals / Non-Goals

**Goals:**
- Any git-backed session whose branch has a GitHub PR gets auto-populated PR metadata
- Detection happens lazily on session enter or browser selection (not eagerly on every list)
- Once detected, the existing 5-minute refresh cycle takes over
- Zero impact when `gh` is not installed — silently skipped

**Non-Goals:**
- Detecting PRs on non-GitHub remotes (GitLab, Bitbucket)
- Eagerly scanning all sessions for PRs (too slow, N `gh` calls)
- Creating PRs from ez (out of scope)

## Decisions

### 1. Detect on session enter, not on list/browse

Running `gh pr list` for every session during a list render would be O(N) network calls. Instead, detection runs once when a session is entered (or selected in the browser), which is O(1) per interaction. The result is persisted in the session env, so subsequent renders show it from cache.

### 2. Use `gh pr list --head <branch>` instead of `gh pr view`

`gh pr view` requires a PR number or URL. `gh pr list --head <branch>` finds PRs by branch name, which is exactly what we have for sessions created without PR metadata. We take the first result (most recent PR for that branch).

### 3. Skip default sessions and bare sessions

The default/main session tracks the main branch and will almost never have an open PR against it. Bare sessions have no worktree and no branch. Skipping these avoids unnecessary `gh` calls.

### 4. Persist immediately after detection

Once detected, the PR metadata (`ez_pr_number`, `ez_pr_url`, `ez_pr_status`) is written to the session env and saved to disk. This means the detection only runs once per session lifetime — subsequent enters use the existing refresh logic.

## Risks / Trade-offs

- **`gh` latency on first enter** → The `gh pr list` call adds ~500ms on the first session enter. Acceptable since it's a one-time cost per session and runs in the foreground where the user is already waiting for tmux attach.
- **False negatives** → If the branch doesn't have a PR yet at first enter, detection won't find one. The next enter will retry since there's no `ez_pr_number` env. Once a PR is created and the user re-enters, it will be detected.
- **Branch name ambiguity** → Multiple PRs could exist for the same branch (force-pushed, closed and reopened). We take the first result from `gh pr list` which is the most recent.
