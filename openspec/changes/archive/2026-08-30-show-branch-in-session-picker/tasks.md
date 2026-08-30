## 1. Shared Helper

- [x] 1.1 Add `format_branch_indicator(branch: Option<&str>) -> String` in `src/browser/mod.rs` — returns `" (branch-name)"` dimmed when `Some`, empty string when `None`

## 2. Session Picker (`session_action_loop`)

- [x] 2.1 In `session_action_loop` (browser/mod.rs), resolve the branch for each session: use `get_branch()` on `session.path` if set, otherwise on `repo_entry.path`
- [x] 2.2 Insert the branch indicator into the session display string after the session name (before `marker`, `bare_indicator`, `pr_indicator`, `labels`, `last_used`)

## 3. Tree View

- [x] 3.1 In `browser/views/tree.rs`, resolve the branch for each session node using the same logic (session path or repo path fallback)
- [x] 3.2 Insert the branch indicator into the tree view session display string after the session name

## 4. Verification

- [x] 4.1 Run `make check` — ensure zero warnings, all tests pass, formatting correct
- [x] 4.2 Update `AGENTS.md` and `docs/user-guide.md` to mention branch display in session picker
