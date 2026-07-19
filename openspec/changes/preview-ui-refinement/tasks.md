## 1. Preview refinements

- [x] 1.1 In `preview_repo`, skip the Sessions section and repo labels when `show_actions` is true
- [x] 1.2 In `preview_repo`, add PR status to Git Info section by loading sessions and finding ones with `ez_pr_number`/`ez_pr_status` metadata for the current branch
- [x] 1.3 In `preview_non_git_repo`, skip the Sessions section and repo labels when `show_actions` is true

## 2. Build and verify

- [x] 2.1 Run `make build` — ensure zero warnings
- [x] 2.2 Run `make test` — ensure all tests pass
