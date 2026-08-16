## Why

The "From GitHub PR" name builder mode only accepts a full PR URL (`https://github.com/org/repo/pull/42`). When creating a session from inside a repo, the owner/repo part of that URL is redundant — the repo is already known. Users who have the PR number in hand (from a review notification, `gh pr list`, or a teammate saying "take a look at 1234") must first go find the full URL just to paste it back.

## What Changes

- The "From GitHub PR" prompt accepts a **bare PR number** (`42`, `#42`) in addition to a full PR URL
- When a bare number is entered, the GitHub repo is auto-resolved from the current repo's remote by running `gh` with the repo root as its working directory (`gh` resolves the remote itself, honoring `gh repo set-default`)
- The canonical PR URL is read back from `gh` (`--json url`) and stored in `ez_pr_url`, so number-entered PRs get the same env metadata as URL-entered ones
- The prompt label and mode description are updated to advertise both accepted forms
- Repo context (repo root path) is threaded from the callers (`ez session new`, browser new-session keybinds) into the name builder so the number form can resolve
- If the repo has no GitHub remote, `gh` is missing, or the PR does not exist, the system falls back to the existing `pr<number>` behavior with a warning — same as today's URL failure path
- Non-goal: no new CLI flag (`--pr`) and no dedicated browser keybind; this is interactive-mode only

## Capabilities

### New Capabilities

(none — this extends existing capabilities)

### Modified Capabilities
- `name-builder-modes`: "From GitHub PR" mode accepts a bare PR number as well as a URL, resolving the repo from the session's repo context
- `pr-checkout`: PR resolution via `gh` runs with the repo root as working directory and reads the canonical `url` back from `gh` when the input was a bare number

## Impact

- `src/session/name_builder.rs`: `prompt_session_name` / `prompt_session_name_default` gain a repo-context parameter; `prompt_github_pr` accepts number input; `resolve_pr_via_gh` takes a working directory and requests `url` from `gh`
- `src/browser/mod.rs`: three `prompt_session_name` call sites pass `repo_entry.path`
- `src/session/mod.rs`: `new_session` passes the resolved repo path into the name prompt
- Requires `gh` CLI for the number form (already an optional dependency for the URL form)
- No breaking changes — pasting a full URL keeps working exactly as before, including outside a GitHub repo
