## Why

Creating a session from a GitHub PR sets `ez_start_point` to `origin/<headRefName>`, but the git-worktree plugin only fetches inside `resolve_default_start_point()` — which the start-point override path skips entirely. If the PR branch was pushed after the last local fetch (the common case for a PR someone else just opened), `origin/<headRefName>` does not exist locally and `git worktree add` fails with "invalid reference", leaving a registered session with no worktree.

## What Changes

- The git-worktree plugin fetches the override start point's ref from `origin` before using it, so a remote branch the local clone has never seen resolves.
- The fetch-TTL cache does not suppress this fetch: the TTL guards the periodic `origin/main` refresh, not a specific ref that may simply be absent.
- The pre-set-session-path branch of the plugin (another `mutates_session_path` plugin already chose the directory) honors `ez_start_point` too — today it ignores the override and resolves the default start point instead.
- Worktree creation reports a clear error when the override ref still cannot be resolved after fetching, instead of a raw git message.

Non-goal: PRs opened from forks. Their head branch does not exist on `origin` at all, so `origin/<headRefName>` is unresolvable regardless of fetching; that needs `refs/pull/<n>/head` handling and is left for a separate change.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `pr-checkout`: the "Start point override for PR branch" requirement gains the guarantee that the override ref is fetched before worktree creation, and that the override applies on the pre-set-session-path route.

## Impact

- `plugins/git-worktree/git-worktree-plugin` — start-point resolution and fetch sequencing.
- No Rust changes: `ez_start_point` is already set correctly by `session::name_builder::PrMetadata::to_session_env`.
- Session creation from a PR gains one single-ref `git fetch` (fast); non-PR session creation is unaffected.
