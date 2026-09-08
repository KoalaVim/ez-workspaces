## 1. Plugin start-point resolution

- [x] 1.1 In `plugins/git-worktree/git-worktree-plugin`, add `resolve_start_point_with_override()`: when `$START_POINT_OVERRIDE` is non-empty, fetch its ref (`git -C "$REPO_PATH" fetch origin "${START_POINT_OVERRIDE#origin/}"`, ignoring exit status) if it starts with `origin/`, then echo the override; otherwise delegate to `resolve_start_point()`. Log both branches via `dbg`.
- [x] 1.2 Replace the inline `if [ -n "$START_POINT_OVERRIDE" ]` block in the `on_session_create` case (around line 212) with a call to `resolve_start_point_with_override()`.
- [x] 1.3 In the pre-set-session-path route (around line 163), call `resolve_start_point_with_override()` instead of `resolve_start_point()` so the override is honored there too — note this route recomputes `REPO_PATH`/`FETCH_STAMP` first, so the fetch targets the session's own clone.
- [x] 1.4 After resolving an override start point, verify it with `git -C "$REPO_PATH" rev-parse --verify "$START_POINT"`; on failure call `json_error` naming the unresolved start point and exit 0 (matching the existing error convention) instead of falling through to `worktree add`.

## 2. Verify

- [x] 2.1 Write a throwaway check in the scratchpad: create a bare origin repo plus two clones, push a branch from clone A, then pipe an `on_session_create` request JSON with `session.start_point = origin/<branch>` into the plugin from clone B (which has never fetched it) and assert the worktree exists at the branch head.
- [x] 2.2 Run the same check with `session.start_point = origin/does-not-exist` and assert the response is `{"success": false, ...}` with the start point named in the error, and that no worktree directory is left behind.
- [x] 2.3 Sanity-check a non-override create (no `start_point`) still fetches and branches from `origin/main`, and that a raw-SHA override (the `session-from-dirty` shape) skips the fetch and still resolves.
- [ ] 2.4 End-to-end: create a session from a PR opened after the last local fetch via the name builder's "From GitHub PR" mode and confirm the worktree lands on the PR head.

## 3. Docs and specs

- [x] 3.1 If `docs/plugin-guide.md` documents `start_point` handling, note that an `origin/` override is fetched before use.
- [x] 3.2 Run `openspec validate --change fetch-pr-start-point` and sync the `pr-checkout` delta into `openspec/specs/pr-checkout/spec.md` when implementation is done.
