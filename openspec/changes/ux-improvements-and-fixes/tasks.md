## 1. Bug Fix: Tree View Session Enter

- [x] 1.1 Add `NodeKind` enum to `browser/views/tree.rs` with `Root`, `Repo(RepoEntry)`, and `Session { repo_entry, session, target_dir }` variants
- [x] 1.2 Refactor the `nodes` vector in `tree::run` to store `NodeKind` alongside the display/preview data
- [x] 1.3 On `ActionResult::Select`, dispatch by `NodeKind`: `Root` → re-render, `Repo` → call `session_action_loop`, `Session` → call `accept_session` with `post_cmd_file`
- [x] 1.4 Pass `post_cmd_file` into `tree::run` (currently ignored as `_post_cmd_file`)

## 2. Bug Fix: Default New Sessions Under Main

- [x] 2.1 Add `find_default(&self) -> Option<&Session>` to `SessionTree` in `session/tree.rs` (returns the session with `is_default == true`)
- [x] 2.2 In `session/mod.rs::new_session`, when `parent` is `None`, default `parent_id` to `tree.find_default().map(|s| s.id.clone())`
- [x] 2.3 In `session/mod.rs::register_existing_worktree`, apply the same default `parent_id` logic when `parent` is `None`
- [x] 2.4 In `session/mod.rs::create_child_session`, verify it already receives an explicit `parent_id` (no change needed — confirm only)

## 3. Bug Fix: Back Navigation

- [x] 3.1 In `browser/views/workspace.rs`, after `browse_repo` returns, return `Outcome::Switch(ViewMode::Workspace)` instead of `Outcome::Done`
- [x] 3.2 In `browser/views/repo.rs`, after session action loop returns on Escape, return `Outcome::Switch(ViewMode::Repo)`
- [x] 3.3 In `browser/views/owner.rs`, after session action loop returns, return `Outcome::Switch(ViewMode::Owner)`
- [x] 3.4 In `browser/views/label.rs`, after session action loop returns, return `Outcome::Switch(ViewMode::Label)`
- [x] 3.5 In `browser/views/tree.rs`, after `session_action_loop` returns for a repo node, return `Outcome::Switch(ViewMode::Tree)`
- [x] 3.6 Audit `drill_into_directory` in `browser/mod.rs` — confirm Escape in drill-down already returns to parent (it does via `history.pop()`)

## 4. Bug Fix: Tmux Session Kill Reliability

- [x] 4.1 In `session/mod.rs::reap_delete`, add a 200ms sleep before invoking plugin hooks
- [x] 4.2 In `spawn_detached_reap`, clear `TMUX` env var on the child command so tmux commands target the server directly
- [x] 4.3 Add `log::debug!` instrumentation to `reap_delete` for hook invocations and outcomes
- [x] 4.4 In the tmux plugin's `on_session_delete` handler, add retry logic: if `tmux kill-session` fails, wait 500ms and retry once
- [x] 4.5 Add `reap_delay_ms` config option to `[plugin_settings.tmux]` (default: 200)

## 5. Feature: Cd Keybind in Session Picker

- [x] 5.1 Add `cd_session` field to `KeybindsConfig` in `config/model.rs` (default: `"alt-c"`)
- [x] 5.2 Register `cd_session` in `expect_keys` in `session_action_loop`
- [x] 5.3 Add handler in `session_action_loop` that calls `write_cd_target(cd_file, &target_dir)` directly
- [x] 5.4 Add `cd_session` keybind to the session picker header display
- [x] 5.5 Update the preview pane session-actions help to include the cd keybind
- [x] 5.6 Document the keybind in README.md browser keybinds table

## 6. Feature: Tree Glyphs in Session Picker

- [x] 6.1 Extend `SessionTree::render_tree()` to return an `is_last_sibling` flag per node (or compute it from the flat list)
- [x] 6.2 Create a `format_session_tree_line` helper that renders box-drawing connectors based on depth, `is_last_sibling`, and ancestor continuation flags
- [x] 6.3 Replace `"  ".repeat(depth)` indentation in `session_action_loop` with the tree glyph renderer
- [x] 6.4 Add unit tests for tree glyph rendering: single child, multiple siblings, deep nesting, empty levels

## 7. Feature: Name Builder Mode Selection

- [x] 7.1 Add `NameBuilderMode` enum (`FullName`, `BuildFromParts`, `GitHubPr`, `JiraUrl`) to `config/model.rs`
- [x] 7.2 Add `name_builder_modes` field to `EzConfig` (default: all four modes)
- [x] 7.3 Maintain backward compat: if `session_name_stages` exists at top-level, use it with deprecation warning
- [x] 7.4 Add `select_mode` function in `session/name_builder.rs` that presents configured modes via `select_one`
- [x] 7.5 Refactor `prompt_session_name` to call `select_mode` first, then dispatch to mode handler
- [x] 7.6 Implement `FullName` mode: single `input_with_back` prompt, reject empty
- [x] 7.7 Keep `BuildFromParts` mode as the existing staged builder (extract into its own function)
- [x] 7.8 Skip mode picker when only one mode is configured

## 8. Feature: GitHub PR Mode

- [x] 8.1 Implement `prompt_github_pr` in `session/name_builder.rs`: prompt for URL via `input_with_back`
- [x] 8.2 Parse PR URL with regex `github\.com/[^/]+/[^/]+/pull/(\d+)` → extract PR number, default name `pr<number>`
- [x] 8.3 Add `OnNameResolve` hook type to `plugin/model.rs::HookType` enum
- [x] 8.4 Add `NameResolveContext` to `plugin/protocol.rs` with `raw_url` and `candidate_name` fields
- [x] 8.5 Implement hook dispatch in `plugin/mod.rs`: call enabled plugins with `OnNameResolve`, use returned name if provided
- [x] 8.6 In `prompt_github_pr`, call `OnNameResolve` hook after parsing — if plugin returns a branch name, use it as session name
- [x] 8.7 Handle plugin timeout/failure: fall back to `pr<number>` with a log message
- [x] 8.8 Show "Resolving PR branch..." message during hook execution

## 9. Feature: Jira URL Mode

- [x] 9.1 Implement `prompt_jira_url` in `session/name_builder.rs`: prompt for URL via `input_with_back`
- [x] 9.2 Parse Jira URL with regex `/browse/([A-Z][A-Z0-9]+-\d+)` → extract ticket key (e.g. `PROJ-123`)
- [x] 9.3 Extract the final descriptive-name prompt from `prompt_session_name` into a reusable `prompt_final_suffix` function
- [x] 9.4 After extracting ticket key, call `prompt_final_suffix` for optional description
- [x] 9.5 Join as `PROJ-123-description` or just `PROJ-123` if no suffix

## 10. Feature: Interactive Builder Flag

- [x] 10.1 Add `--interactive` / `-i` flag to `SessionCommand::New` in `cli.rs`
- [x] 10.2 Pass the flag through `session::dispatch` to `new_session`
- [x] 10.3 In `new_session`, when `interactive` is true, skip the provided name and enter the mode selection + name builder flow

## 11. Feature: Enhanced Branch Fetch on Session Create

- [x] 11.1 In the git-worktree plugin's `on_session_create` handler, after fetching main/master, also run `git fetch origin <session-name>` for the target branch
- [x] 11.2 If `origin/<session-name>` exists after fetch, create the worktree tracking that remote branch instead of branching from base
- [x] 11.3 If the fetch fails or the remote branch doesn't exist, fall through to current behavior (new branch from base)

## 12. Feature: Return-to-ez After Tmux Detach

- [x] 12.1 Modify the shell wrapper in `main.rs::print_shell_init` to include a loop with sentinel file check (`/tmp/ez-reenter-$$`)
- [x] 12.2 In the tmux plugin's `on_session_enter` (attach) handler, write the sentinel file before calling `tmux attach`
- [x] 12.3 Add `return_on_detach` boolean to tmux plugin config schema (default: `true`); only write sentinel when enabled
- [x] 12.4 Add stale sentinel cleanup in the shell wrapper (remove files older than 1 hour)
- [x] 12.5 Update Fish shell wrapper with equivalent loop logic

## 13. Chore: README Refinement

- [x] 13.1 Update the commands table with new flags (`--interactive`, `--select-by` modes)
- [x] 13.2 Add the `cd_session` keybind to the browser keybinds table
- [x] 13.3 Add a section on name builder modes with examples for each mode
- [x] 13.4 Review and update the Quick Start section for current accuracy
- [x] 13.5 Add a note about return-to-ez after tmux detach behavior

## 14. Documentation and Spec Updates

- [x] 14.1 Update `AGENTS.md` with any new modules, hooks, or config fields
- [x] 14.2 Update `docs/user-guide.md` with new features
- [x] 14.3 Update `docs/design.md` diagrams if module dependencies change
- [x] 14.4 Update `docs/plugin-guide.md` with the `OnNameResolve` hook documentation
