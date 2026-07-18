## Context

The ez-workspaces browser and session flows have accumulated UX bugs and missing features since the initial implementation. The tree view bypasses `accept_session` for session enter, new sessions are orphaned as roots instead of parented under main, back-navigation is broken at view→session transitions, and the tmux kill flow has race conditions. The name builder only supports the staged prefix/ticket/name flow and lacks quick-entry shortcuts or integration with external sources (PR links, Jira URLs).

This design covers all 11 items from the proposal: 4 bug fixes and 7 feature additions.

## Goals / Non-Goals

**Goals:**
- Fix all four identified bugs so the browser is correct end-to-end
- Introduce a mode-selection layer to the name builder that's extensible for future modes
- Add GitHub PR and Jira URL modes as the first two "smart" modes
- Polish the session picker with tree glyphs and a dedicated cd keybind
- Implement return-to-ez after tmux detach for workflow continuity

**Non-Goals:**
- Full async plugin protocol — all new work remains synchronous
- Replacing the fzf backend or abstracting away from it
- Redesigning the plugin hook type system (we'll use existing hooks where possible)
- Changing the session store format (TOML on disk)
- Supporting interactive mode selection for `create_child_session` (browser already has the selector)

## Decisions

### 1. Tree View Session Enter — Use `accept_session`

**Problem:** `tree.rs` line 174 calls `write_cd_target(cd_file, target)` directly when a session node is selected, bypassing `accept_session` which dispatches `on_enter` actions (tmux attach, plugin binds, etc.).

**Approach:** The tree view must identify whether a selected node is a session (vs. a repo or workspace root). Add a `NodeKind` enum to the nodes vector:

```rust
enum NodeKind {
    Root,                      // workspace root — no target
    Repo(PathBuf),             // repo dir — enter session_action_loop
    Session {
        repo_entry: RepoEntry,
        session: Session,
        target_dir: PathBuf,
    },
}
```

On `Select`, dispatch by kind: `Root` → re-render, `Repo` → call `session_action_loop` (enters the session picker for that repo), `Session` → call `accept_session` with the session's `target_dir`, `cd_file`, and `post_cmd_file`.

**Why not just fix the `write_cd_target` call?** Because repos should open the session picker (not cd), and sessions need the full `accept_session` flow. The current code treats all targets uniformly which is fundamentally wrong.

**Alternative considered:** Passing `post_cmd_file` into `write_cd_target` and having it write tmux commands — rejected because it duplicates the `accept_session` logic and doesn't handle plugin binds.

**Back-navigation from tree:** When `session_action_loop` returns (user pressed Escape), the tree view should return `Outcome::Switch(ViewMode::Tree)` to re-render itself, not `Outcome::Done`.

### 2. Default New Sessions Under Main

**Problem:** `new_session` and `register_existing_worktree` set `parent_id = None` when no `--parent` is given, creating root-level siblings of main.

**Approach:** When `parent` is `None`, look up the default session (the one with `is_default: true`) in the loaded tree and use its ID as `parent_id`. This is a two-line change in each function:

```rust
let parent_id = if let Some(parent_name) = parent {
    // existing explicit parent logic
} else {
    tree.find_default().map(|s| s.id.clone())
};
```

Add `find_default(&self) -> Option<&Session>` to `SessionTree` (returns the session with `is_default == true`).

**Why default to main and not "no parent"?** The tree hierarchy is the core mental model. Orphan roots clutter the tree and break the invariant that `main` is the ancestor of all work. Users who want explicit roots can pass `--parent ""` or a future `--root` flag.

**Alternative considered:** Always defaulting to the *current* session as parent — rejected because CLI usage (`ez session new`) often runs outside any session context, and defaulting to "current" would be confusing when there is no current session.

### 3. Back Navigation — Outcome::Switch on Escape from Session Loop

**Problem:** In `workspace.rs` line 100-101, after `browse_repo`/`session_action_loop` returns (user Escaped), the function returns `Outcome::Done`, exiting the entire browser. Same pattern exists in other views.

**Approach:** Change the convention: when a view calls `session_action_loop` or `browse_repo` and the user Escapes out, the view should return `Outcome::Switch(self_mode)` to re-render its own level. Concretely:

- `workspace.rs`: after `browse_repo` returns, return `Outcome::Switch(ViewMode::Workspace)` instead of `Outcome::Done`.
- `repo.rs`, `owner.rs`, `label.rs`: same pattern — Escape from the session loop re-renders the repo/owner/label picker.
- `tree.rs`: after `session_action_loop` returns for a repo node, return `Outcome::Switch(ViewMode::Tree)`.

**Why not add a `Back` variant to `Outcome`?** The dispatch loop in `views/mod.rs` already handles `Switch` correctly. Adding `Back` would require tracking view history in the loop, which adds complexity for no gain when each view already knows what "back" means (itself).

**Alternative considered:** Making `session_action_loop` return a discriminant indicating Escape vs. successful enter — rejected because the current return type is `Result<()>` and changing it to an enum would touch many call sites. Instead, the calling view always re-renders on return (the `Ok(())` case from `session_action_loop` already means "user is done with this repo").

### 4. Tmux Kill Reliability — Timing and Signal Fixes

**Problem:** The detached reap worker (spawned via `spawn_detached_reap`) sometimes fails to kill the tmux session. Root causes to investigate:
1. The reap worker reads the payload file and runs hooks, but `tmux kill-session` may fire before the tmux server has fully processed the detach.
2. The `setsid()` worker may inherit a signal mask or environment that interferes with tmux client detection.

**Approach:** Add a short sleep (200ms) at the start of `reap_delete` before invoking plugin hooks. This gives the tmux server time to fully detach the client before the kill arrives. Additionally:
- Pass `TMUX=""` in the reap worker's environment to ensure tmux commands target the server directly (not the inherited session).
- Add `log::debug!` instrumentation to `reap_delete` for diagnosing future issues.
- In the tmux plugin's `OnSessionDelete` hook, retry `tmux kill-session` once with a 500ms gap if the first attempt fails.

**Why a sleep rather than IPC?** The reap worker is fire-and-forget by design. Adding an IPC channel between the foreground `ez` and the worker would defeat the purpose of the detached architecture (surviving terminal teardown). A small sleep is pragmatic and sufficient.

**Alternative considered:** Running the kill synchronously before spawning the reap worker — rejected because the kill itself can destroy the terminal pane, which would kill the foreground `ez` process before it finishes printing output.

### 5. Name Builder Mode Selection Architecture

**Problem:** The name builder only supports the staged prefix/ticket/name flow. Users want shortcuts (full name, PR URL, Jira URL).

**Approach:** Add a mode selection step before the staged builder. The modes are:

1. **Full name** — single free-text prompt, returns immediately
2. **Build from parts** — existing staged builder (unchanged)
3. **From GitHub PR** — paste URL → extract PR number → optionally fetch branch
4. **From Jira URL** — paste URL → extract ticket key → continue with descriptive suffix

Architecture:

```rust
// config/model.rs
pub struct NameBuilderConfig {
    pub modes: Vec<NameBuilderMode>,
    pub stages: Vec<SessionNameStage>,  // moved from top-level
}

pub enum NameBuilderMode {
    FullName,
    BuildFromParts,
    GitHubPr,
    JiraUrl,
}
```

The mode picker is itself an fzf selection (using `select_one`) shown before any stages. Each mode is a function that takes the selector and config, returns `NamePromptResult`.

**Dispatch:**

```rust
fn prompt_session_name(selector, config) -> Result<NamePromptResult> {
    let mode = select_mode(selector, &config.name_builder.modes)?;
    match mode {
        FullName => prompt_full_name(selector),
        BuildFromParts => prompt_staged(selector, config),  // existing logic
        GitHubPr => prompt_github_pr(selector, config),
        JiraUrl => prompt_jira_url(selector, config),
    }
}
```

**Configuration:** The `session_name_stages` field remains for backward compatibility but is nested under a new `[name_builder]` section. If only one mode is configured, the mode picker is skipped.

**Why an enum rather than a trait/plugin hook for modes?** Modes need tight integration with the selector (multi-step prompts, back navigation) and git operations (branch fetch for PR mode). A plugin hook would require round-tripping complex UI state through JSON, which is impractical. New modes can be added as enum variants.

### 6. GitHub PR Mode — Inline Parsing with Plugin Hook for Branch Resolution

**Problem:** Users want to paste `https://github.com/org/repo/pull/123` and get a session named `pr123` (or the PR's branch name).

**Approach:** Two phases:

**Phase 1 — URL parsing (inline in name_builder):**
- Prompt for URL via `input_with_back`
- Parse with regex: `github\.com/[^/]+/[^/]+/pull/(\d+)` → extract PR number
- Default session name: `pr<number>` (e.g., `pr123`)

**Phase 2 — Branch resolution (plugin hook):**
- Define a new hook type: `OnNameResolve` — called after the name builder produces a candidate name, with the raw URL in context. The hook can return an alternative name (the PR's head branch) and/or set `plugin_state` fields.
- The git-worktree plugin (or a new `github` plugin) implements this hook: calls `gh pr view <number> --json headRefName` to fetch the branch, returns it as the resolved name.
- If the plugin is not enabled or fails, fall back to `pr<number>`.

**Why not a dedicated GitHub plugin?** The branch resolution is optional and the parsing is trivial regex. Keeping parsing inline avoids a plugin round-trip for the common case. The hook is a clean extension point for users who want branch-name sessions.

**Alternative considered:** Using the existing `OnSessionCreate` hook — rejected because by that point the session name is already committed. We need resolution *before* session creation.

**Alternative considered:** A standalone `OnPrResolve` hook — rejected as too specific. `OnNameResolve` generalizes to any URL/input that needs external resolution.

### 7. Jira URL Mode — Regex Parsing with Suffix Continuation

**Problem:** Users paste `https://company.atlassian.net/browse/PROJ-123` and want a session named `PROJ-123-short-description`.

**Approach:**
- Prompt for URL via `input_with_back`
- Parse with regex: `/browse/([A-Z][A-Z0-9]+-\d+)` or path segment matching `[A-Z]+-\d+`
- Extract the ticket key (e.g., `PROJ-123`)
- Set it as the "ticket" part, then continue to the final descriptive-name stage from the staged builder (reusing that UI)
- Final name: `PROJ-123-description`

**Why not extract from the page title via HTTP?** Adding HTTP dependencies (reqwest, auth) is out of scope and would break the synchronous, no-network-required design of `ez`. The ticket key from the URL is sufficient context.

**Integration with staged builder:** The Jira mode calls `prompt_final_suffix(selector)` — a refactored extraction of the last stage from `prompt_session_name` — and prepends the ticket key.

### 8. Enhanced Branch Fetch During Session Create

**Problem:** The git-worktree plugin creates a branch from `origin/main` but doesn't fetch the specific branch name first. If the branch exists remotely but is stale locally, the worktree may be based on old content.

**Approach:** In the git-worktree plugin's `OnSessionCreate` hook, before creating the worktree:
1. Run `git fetch origin <branch-name>` if a branch name is known (from PR mode or when the session name matches a remote branch pattern).
2. If the fetch succeeds and `origin/<branch-name>` exists, create the worktree tracking it instead of branching from `origin/main`.
3. If the fetch fails or the branch doesn't exist remotely, fall through to the current behavior (new branch from base).

This is purely a plugin-level change — no core changes needed.

**Why fetch by name rather than `git fetch --all`?** Fetching all refs is expensive for large repos and fetches irrelevant data. Targeted fetch is fast and sufficient.

### 9. Cd Keybind in Session Picker

**Problem:** When `on_enter` is configured to `tmux` (attach to tmux session), there's no way to "just cd" into a session's worktree directory from the picker.

**Approach:** Add a `cd_session` keybind to `KeybindsConfig` (default: `alt-c`). In `session_action_loop`, register it in `expect_keys` and handle it:

```rust
key if key == keybinds.cd_session => {
    let target_dir = selected.path.clone().unwrap_or(repo_entry.path.clone());
    return write_cd_target(cd_file, &target_dir);
}
```

This bypasses `accept_session` entirely and always writes the cd target, regardless of `on_enter` config.

**Why a dedicated keybind rather than a modifier on Enter?** Modifiers on Enter aren't reliably detectable in fzf's `--expect` mechanism. A separate keybind is explicit and discoverable via the header.

### 10. Tree Glyphs in Session Picker

**Problem:** The session picker in `session_action_loop` uses indentation (`"  ".repeat(depth)`) for hierarchy but lacks visual tree connectors.

**Approach:** Replace depth-based indentation with box-drawing characters, matching the style in `tree.rs`. Use the `render_tree()` output which already provides `(depth, session)` pairs, and compute connectors based on sibling position:

```rust
fn format_session_tree_line(
    session: &Session,
    depth: usize,
    is_last_sibling: bool,
    ancestor_continuation: &[bool],  // which ancestor levels need a │ line
) -> String
```

The rendering needs to know which nodes at each depth are "last children" to choose `└──` vs `├──`. Extend `SessionTree::render_tree()` to also return an `is_last` flag per node, or compute it in the session_action_loop from the flat rendered list by checking if the next node has the same or lesser depth.

**Why not use the existing tree.rs rendering?** The tree view renders repos *and* sessions across all workspace roots. The session picker only shows sessions for a single repo. The logic is similar but the context is different — share the glyph constants but keep the rendering separate.

### 11. Return-to-ez After Tmux Detach

**Problem:** After `Ctrl-b d` (tmux detach), the user lands back in their original shell with no ez context. They must manually re-run `ez` to pick another session.

**Approach:** Wrap the `ez` invocation in a loop at the shell-integration level. The `ez init-shell` output already generates a shell function. Modify it to:

```bash
ez() {
    while true; do
        command ez "$@"
        local rc=$?
        [ -f /tmp/ez-reenter-$$ ] && rm /tmp/ez-reenter-$$ && continue
        break
    done
    return $rc
}
```

The tmux plugin's `OnSessionEnter` (attach) hook writes `/tmp/ez-reenter-$$` before attaching. When tmux detach occurs, `tmux attach` exits, `ez` exits normally, and the shell wrapper sees the sentinel file and re-enters `ez`.

**Why a sentinel file rather than an exit code?** Exit codes are fragile (other errors could produce the same code) and hard to distinguish from legitimate "done" exits. A file is explicit and only written when the intent is to re-enter.

**Why not a background process watching for detach?** That would require polling tmux state or using tmux hooks (which are per-server, not per-client), adding complexity. The sentinel approach requires zero background processes.

**Configuration:** Add a `return_on_detach` boolean to the tmux plugin settings (default: `true`). When false, the sentinel is not written and detach behaves as before.

## Risks / Trade-offs

### Backward Compatibility of Name Builder Config

**Risk:** Moving `session_name_stages` under a new `[name_builder]` section breaks existing configs.

**Mitigation:** Support both locations during a deprecation period. If `session_name_stages` exists at the top level, use it and emit a one-time warning. The new `[name_builder]` section takes precedence if both are present.

### OnNameResolve Hook Adds Latency

**Risk:** Calling `gh pr view` adds network latency (1-3s) to session creation from PR mode.

**Mitigation:** The hook is only invoked in PR mode (not the default flow). Show a spinner/message ("Resolving PR branch...") during the call. If the hook times out (respects `plugin_timeout`), fall back to `pr<number>`.

### Tree Glyph Rendering Complexity

**Risk:** Computing `is_last_sibling` from the flat `render_tree()` output requires traversal logic that could have off-by-one bugs.

**Mitigation:** Add unit tests for tree glyph rendering with various topologies (single child, multiple children, deep nesting, empty levels). The logic is deterministic and testable without fzf.

### Return-to-ez Sentinel Race Condition

**Risk:** If `ez` crashes between writing the sentinel and attaching to tmux, the next shell invocation re-enters `ez` unexpectedly.

**Mitigation:** The sentinel uses the shell's PID (`$$`) so it's scoped to the current shell session. Additionally, the tmux plugin should write the sentinel *only* if `tmux attach` is about to be called (not on failure paths). Clean up stale sentinels older than 1 hour via the shell wrapper.

### Detached Reap Worker Timing

**Risk:** The 200ms sleep is heuristic. On slow machines or loaded systems, tmux may need more time.

**Mitigation:** Make the delay configurable via `plugin_settings.tmux.reap_delay_ms` (default: 200). The retry in the tmux plugin's hook provides a second chance if the first kill fails.

### Tree View NodeKind Refactor

**Risk:** Adding `NodeKind` with `RepoEntry` and `Session` to the nodes vector increases memory usage and changes the data flow.

**Mitigation:** The tree view already clones repo paths and names into the vector. Adding the full `RepoEntry`/`Session` is a modest increase (these are small structs). The alternative of re-loading them on selection would add I/O latency.
