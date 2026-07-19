## Why

PR status tracking currently only works for sessions created via the GitHub PR name builder mode (`ez session new -i` → GitHub PR → paste URL). Sessions created through any other method — manual name, parts builder, Jira mode, or `ez session register` — never get PR metadata, even when their branch has an associated PR on GitHub. This means the majority of sessions miss out on PR status indicators in the browser and preview pane.

## What Changes

- Auto-detect PR association for any session whose branch has an open PR on GitHub
- On session enter (or first browse after creation), if a session has no `ez_pr_number` in its env, run `gh pr list --head <branch> --json number,url,state` to discover an associated PR
- Populate `ez_pr_number`, `ez_pr_url`, and `ez_pr_status` in the session env when a PR is found
- Once populated, the existing refresh logic (stale after 5 min) continues to work as before
- Skip detection for bare sessions, non-git repos, and the default/main session
- Detection is best-effort: silently skipped if `gh` is not installed or not authenticated

## Capabilities

### New Capabilities

### Modified Capabilities
- `session-management`: Add auto-detection of PR association on session enter for sessions without existing PR metadata

## Impact

- `src/session/mod.rs`: Add `detect_pr_for_session` function called alongside existing `refresh_pr_status` on session enter
- `src/browser/mod.rs`: Call detection when entering a session from the browser
- Requires `gh` CLI (best-effort, already an optional dependency)
- No breaking changes — sessions with existing PR metadata are unaffected
