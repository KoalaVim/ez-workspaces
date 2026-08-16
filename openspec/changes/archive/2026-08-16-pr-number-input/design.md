## Context

`prompt_github_pr` in `src/session/name_builder.rs` prompts for a GitHub PR URL and matches it against `github\.com/[^/]+/[^/]+/pull/(\d+)`. On match it calls `resolve_pr_via_gh(url, pr_number)`, which shells out to `gh pr view <url> --json headRefName,number` (no working directory set — the URL is self-contained) and returns `(session_name, Option<PrMetadata>)`. `PrMetadata::to_session_env` writes `ez_pr_number`, `ez_pr_url`, `ez_pr_status`, and `ez_start_point`.

The name builder currently takes only `(selector, config)`. Its three browser call sites (`src/browser/mod.rs:498/540/579`) already have `repo_entry` in scope, and `new_session` (`src/session/mod.rs:262`) has `repo_entry` resolved before it prompts — so repo context is available at every call site and just isn't threaded through.

Elsewhere in the codebase (`detect_pr_for_session`) `gh` is already invoked with `.current_dir(&repo_entry.path)` to let `gh` resolve the remote itself, so this pattern is established.

## Goals / Non-Goals

**Goals:**
- A bare PR number (`42`, `#42`) is accepted anywhere a PR URL is accepted today
- The GitHub repo for a bare number is resolved from the repo the session is being created in, with no config
- Number-entered PRs produce exactly the same session env as URL-entered ones, including a real `ez_pr_url`
- Failure modes (no `gh`, no GitHub remote, bad number) degrade to today's `pr<number>` fallback

**Non-Goals:**
- A non-interactive `--pr <n>` CLI flag or a dedicated browser keybind (explicitly deferred)
- Cross-repo PR numbers (`org/repo#42` shorthand)
- Non-GitHub forges

## Decisions

### 1. Let `gh` resolve the remote via working directory, not by parsing `git remote`

Running `gh pr view <number>` with `current_dir` set to the repo root makes `gh` do the remote resolution: it reads `remote.origin.url`, honors `gh repo set-default` for multi-remote/fork setups, and handles SSH/HTTPS/enterprise URL forms. Parsing `git remote get-url origin` ourselves and passing `--repo owner/name` would duplicate that logic and get fork setups wrong (a fork's `origin` is usually not where the PR lives, but `gh` already knows the user's default).

Alternative considered: `--repo <owner>/<name>` derived from the remote URL. Rejected — more code, worse behavior on forks.

### 2. Thread repo context as `Option<&Path>`, not a required argument

`prompt_session_name` gains a `repo_dir: Option<&Path>` parameter (and `prompt_session_name_default` mirrors it). `None` means "no repo context" — the URL form still works, and a bare number is rejected with a message telling the user to paste a full URL. This keeps the builder usable from any future call site that lacks a repo without forcing a fake path.

### 3. One resolution path for both input forms

`prompt_github_pr` normalizes input into a `gh` ref plus a fallback number:
- URL match → ref is the URL, number from the capture
- `^#?(\d+)$` match → ref is the number string, number from the capture (requires `repo_dir`)
- otherwise → error and re-prompt

`resolve_pr_via_gh(ref, pr_number, repo_dir)` then runs one command for both. This keeps the two forms from drifting in behavior.

### 4. Always read `url` back from `gh`

`--json` gains `url`, and `PrMetadata.pr_url` is taken from the response, falling back to the pasted input if the field is missing. Today the URL form stores the user's pasted string; reading it back normalizes it (strips `#discussion` anchors, `/files` suffixes, trailing slashes) and is the only way the number form can populate `ez_pr_url` at all — which `refresh_pr_status` and the preview pane both consume.

### 5. `#42` accepted, `42` not treated as ambiguous

A bare integer at this prompt is unambiguously a PR reference — the prompt exists only to identify a PR, and a session named `42` can still be created via Full name mode. Accepting the `#` prefix costs one optional character in the regex and matches how PRs are written in commit messages and chat.

## Risks / Trade-offs

- **`gh` latency is now on the interactive path for number input** → Same as the existing URL path (~500ms), and the existing "Resolving PR branch..." spinner line already covers it.
- **Repo's `origin` points at a fork, so `gh` resolves the PR in the fork** → `gh` already applies `gh repo set-default`; users with fork workflows configure that once. The fallback (`pr<number>` + warning) is non-destructive if resolution picks the wrong repo and finds nothing.
- **Signature change ripples to all call sites** → Four call sites, all in-tree, all with `repo_entry` already in scope. Compiler-enforced, so no silent misses.
- **A PR number that exists in a *different* repo silently resolves to the wrong PR** → Only possible if the user is in the wrong repo, which is already visible in the prompt context; the resolved branch name becomes the session name, so a wrong resolution is immediately obvious before the worktree is used.
