## 1. Thread repo context into the name builder

- [x] 1.1 Add `repo_dir: Option<&Path>` parameter to `prompt_session_name` and `prompt_session_name_default` in `src/session/name_builder.rs`, forwarding it to the `GitHubPr` mode handler
- [x] 1.2 Pass `Some(&repo_entry.path)` at the three `prompt_session_name` call sites in `src/browser/mod.rs` (new session, new bare session, session from dirty)
- [x] 1.3 Pass `Some(&repo_entry.path)` at the `prompt_session_name_default` call site in `new_session` (`src/session/mod.rs`)

## 2. Accept a bare PR number

- [x] 2.1 In `prompt_github_pr`, add a `^#?(\d+)$` branch alongside the existing URL regex: on match, require `repo_dir` and use the number string as the `gh` ref; when `repo_dir` is `None`, print an error telling the user to paste a full PR URL and re-prompt
- [x] 2.2 Update the invalid-input error message to name both accepted forms (URL and number), and update the prompt label to "GitHub PR URL or number"
- [x] 2.3 Update the `NameBuilderMode::GitHubPr` display string in `select_mode` to advertise both forms

## 3. Resolve via gh with repo working directory

- [x] 3.1 Change `resolve_pr_via_gh` to take `(pr_ref: &str, pr_number: u64, repo_dir: Option<&Path>)`, set `.current_dir(dir)` when `repo_dir` is `Some`, and request `--json headRefName,number,url`
- [x] 3.2 Populate `PrMetadata.pr_url` from the `url` field of the `gh` response, falling back to the input ref when the field is absent or empty
- [x] 3.3 Verify the `pr<number>` fallback still fires (with the `gh` stderr shown) when `gh` is missing, unauthenticated, or the PR is not found in the resolved repo

## 4. Verification

- [x] 4.1 Add unit tests for the PR reference parsing (URL form, `42`, `#42`, invalid input, number without repo context)
- [x] 4.2 Manually verify inside a GitHub-backed repo: `ez session new -i` → "From GitHub PR" → enter a bare number → session is named after the PR branch and `ez_pr_number` / `ez_pr_url` / `ez_pr_status` / `ez_start_point` are set in `sessions.toml`
- [x] 4.3 Manually verify the URL form still works unchanged, including from a repo with no GitHub remote
- [x] 4.4 Run `make check` — zero warnings, all tests pass
