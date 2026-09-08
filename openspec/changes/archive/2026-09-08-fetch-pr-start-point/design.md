## Context

See proposal.md — Why.

Current state in `plugins/git-worktree/git-worktree-plugin`:

- `should_fetch()` gates fetching on a TTL stamp file (`$GIT_DIR/ez-fetch-stamp`, default 60s).
- `resolve_default_start_point()` is the only place that fetches. It fetches `origin main`, `origin master`, and `origin $SESSION_NAME` in parallel, stamps the TTL, then picks `origin/main` → `origin/master` → `HEAD`.
- `resolve_start_point()` chooses between the default resolution and the parent worktree's HEAD.
- `on_session_create` (line ~212) uses `$START_POINT_OVERRIDE` when set and otherwise calls `resolve_start_point()`. **The override path never fetches.**
- The pre-set-session-path route (line ~155, taken when another `mutates_session_path` plugin already chose the directory) calls `resolve_start_point()` unconditionally and never looks at `$START_POINT_OVERRIDE`.

For a PR session the override is `origin/<headRefName>` and the branch name is `pr<n>-<headRefName>`, so the existing `origin/${BRANCH_NAME}` tracking check never matches either — nothing in the flow brings the PR's ref into the local clone.

## Goals / Non-Goals

**Goals:**

- One place resolves the start point, and that place fetches whatever ref it is about to hand to `git worktree add`.
- Both create routes behave identically with respect to the override.

**Non-Goals:**

- Fork PRs (`refs/pull/<n>/head`) — see proposal.md.
- Any change to the TTL semantics for the default `origin/main` refresh.
- Fetching arbitrary non-`origin/` override values (e.g. a raw SHA or a local branch); those are used as-is.

## Decisions

**Fix in the plugin, not in Rust.** The Rust side already produces the right `ez_start_point`; git work belongs to the plugin that owns worktree creation. Alternative — fetching from `session::create_child_session` before running hooks — would duplicate git logic in the core and run even for repos whose worktree plugin is disabled.

**Add one `resolve_start_point_with_override()` wrapper and call it from both routes.** The override check moves out of the `on_session_create` case body into the wrapper, so the pre-set-path route picks the behavior up for free. Alternative — patching the fetch into the `on_session_create` case body only — leaves the pre-set-path route broken, which is the sibling-caller bug the current code already has.

**Fetch only the single ref named by the override, and bypass the TTL.** `git -C "$REPO_PATH" fetch origin "${OVERRIDE#origin/}"` is one ref over the wire, so it is cheap enough to run unconditionally on PR session creation; and the TTL stamp says nothing about whether *this* ref was ever fetched. Alternative — a full `git fetch origin` — is much slower on large repos for no added benefit. Alternative — honoring the TTL — reintroduces the bug for any PR opened within the TTL window of an unrelated fetch.

**Fetch failure is not fatal on its own; an unresolvable start point is.** A fetch can fail for reasons that do not matter (offline, but the ref is already local). So the fetch's exit status is ignored, and correctness is enforced by a `git rev-parse --verify` on the override afterwards: if it fails, emit `json_error` naming the start point. Alternative — falling back to `resolve_default_start_point()` — would silently create a worktree from `main` for a session the user asked to open at a PR, which is worse than a clear failure.

## Risks / Trade-offs

- **One extra single-ref fetch on every PR session create, even when the ref is already current** → Single-ref fetch against a warm remote is on the order of the existing fetch already logged in the create path; the create flow already prints fetch timings to stderr, so the cost stays visible.
- **`origin` hardcoded as the remote name** → Already the case throughout this plugin (`origin/main`, `origin/master`, the tracking check); consistent with existing behavior rather than a new assumption.
- **Failing loudly on an unresolvable override surfaces fork PRs as an error instead of a working-but-wrong worktree** → Intended; the error names the ref, and fork support is a follow-up change.
- **A `ez_start_point` set by something other than PR checkout (e.g. `session-from-dirty`) also flows through the wrapper** → `from_dirty` sets a local commit/branch, not an `origin/` ref, so the fetch branch is skipped and only the `rev-parse` verification applies — which it passes.
