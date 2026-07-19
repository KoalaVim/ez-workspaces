## 1. Core Detection Logic

- [x] 1.1 Add `detect_pr_for_session` function in `src/session/mod.rs` — given a session ID and tree, check if session is git-backed, non-bare, non-default, and has no `ez_pr_number`; if so, get the branch name from the worktree path and run `gh pr list --head <branch> --json number,url,state --limit 1`; on success, populate `ez_pr_number`, `ez_pr_url`, `ez_pr_status` in the session env
- [x] 1.2 Call `detect_pr_for_session` in `enter_session` (in `src/session/mod.rs`) before the existing `refresh_pr_status` call — if detection populated env vars, skip refresh; persist tree to disk after detection

## 2. Browser Integration

- [x] 2.1 Call `detect_pr_for_session` in `accept_session` / `session_action_loop` in `src/browser/mod.rs` when a session is selected, before rendering the PR indicator — persist tree after detection

## 3. Verification

- [x] 3.1 Run `make check` — zero warnings, all tests pass
