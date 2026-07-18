# Design: workflow-enhancements

## Context

ez-workspaces currently assumes all tracked repos are git repos, sessions always create worktrees, browser sorting is alphabetical-only, and there's no workflow to move dirty changes between sessions. These four features address the most common gaps users encounter.

The changes span `repo/`, `session/`, `browser/`, `config/`, and the `git-worktree` bundled plugin. They are independent enough to implement and ship incrementally, but share data-model changes (new fields on `RepoEntry`, `RepoMeta`, `Session`, and the plugin protocol's `SessionInfo`/`RepoInfo`).

## Goals / Non-Goals

**Goals:**
- Track non-git directories as first-class repos, with plugins gracefully skipping git operations.
- Sort repos and sessions by last-recently-used, togglable from the browser.
- Create sessions without triggering worktree creation (bare sessions).
- Move uncommitted changes to a new session via stash-based workflow.

**Non-Goals:**
- Supporting non-fzf selectors for the new keybinds (follow existing pattern: add to fzf, abstract later).
- Automatic LRU garbage collection or session expiry.
- Partial-stash (only specific files). The entire working-tree diff is moved.
- Non-git VCS (Mercurial, etc.). "Non-git" means "no VCS at all."

---

## Decisions

### 1. Non-git directory tracking

**Data model.** Add `is_git: bool` to `RepoEntry` (default `true` for backward compat via `#[serde(default = "default_true")]`). This is the canonical flag — it's set at `ez add` / `ez clone` time and persisted in `repos.toml`.

Why `RepoEntry` and not `RepoMeta`: the git/non-git distinction is an intrinsic property of the registration, not per-repo metadata that evolves over time. `RepoMeta` stores things like labels and plugin state that change independently.

**Registration flow.** `ez add <path>`:
- If `<path>/.git` exists → `is_git = true` (current behavior).
- If no `.git` → `is_git = false`, skip remote-URL / default-branch detection, store `RepoMeta` with empty `remote_url` and `default_branch`.
- `ez clone` always produces `is_git = true`.

**Plugin protocol.** Add `is_git: bool` to `RepoInfo` in the hook request. The git-worktree plugin checks this field and returns `{"success": true}` immediately when `false`. This is cleaner than having each plugin probe the filesystem — the source of truth is the registration, not the directory state.

**Sessions under non-git repos.** Session creation skips the name-builder modes that require git (GitHubPr, JiraUrl). The session `path` is set to the repo path directly (no worktree). Session delete doesn't attempt worktree removal. Session enter/cd works normally.

**Browser.** Non-git repos display without branch/worktree info in the preview pane. The owner view excludes non-git repos (no remote URL to parse).

### 2. LRU sorting

**Timestamps.** Add `last_accessed_at: Option<DateTime<Utc>>` to both:
- `RepoMeta` — updated when a user enters any session under the repo, or browses into the repo in the browser.
- `Session` — updated when the session is entered (`ez session enter` or Enter in picker).

Both use `Option` with `#[serde(default)]` for backward compat with existing TOML files.

**What counts as "access."** Only explicit entry: `ez session enter`, Enter/accept in the picker, or `ez browse` drilling into a repo. Browsing past an item in fzf (scrolling) does not count — that would make every browse session update dozens of timestamps.

**Sorting.** A new `SortMode` enum (`Alpha`, `Lru`) drives sorting in all browser views. Sessions and repos are sorted by `last_accessed_at` descending (most recent first), with `None` timestamps sorted last.

**Config.** Add `default_sort: String` to `EzConfig` (default `"alpha"`). Accepts `"alpha"` or `"lru"`.

**Browser toggle.** A new keybind (default `ctrl-s`) switches between Alpha ↔ LRU within the current fzf session. The current sort mode is shown in the fzf prompt/header. Add `sort_toggle: String` to `KeybindsConfig`.

**Persistence.** `RepoMeta` already persists to `meta.toml` per-repo. `Session` persists in `sessions.toml`. The timestamp writes happen after the enter/browse action completes — no new files or stores needed.

### 3. Bare sessions

**Data model.** Add `bare: bool` to `Session` (default `false`, `#[serde(default)]`).

**CLI.** `ez session new --bare` — creates the session record but does not fire `OnSessionCreate` hooks. This is the simplest and most correct approach: bare means "no plugin side-effects at creation."

Why skip the hook entirely rather than adding a `bare` field to `SessionInfo` and letting plugins decide: the hook-skip approach is simpler, doesn't require protocol changes, and matches the semantics — a bare session has no worktree by definition. If a future plugin needs to act even on bare sessions, we can add an `on_session_create_bare` hook or a `skip_hooks: bool` per-hook config.

**Browser.** `Alt-Shift-N` triggers bare session creation. This uses the same name-builder flow as `Alt-N` but skips hooks. Add `new_bare_session: String` to `KeybindsConfig` (default `"alt-shift-n"`).

**Session path.** A bare session's `path` remains `None` (no plugin sets it). `ez session enter` on a bare session cd's to the repo root. Display in tree view shows a `[bare]` indicator.

**Deletion.** `ez session delete` on a bare session skips `OnSessionDelete` hooks (no worktree to clean up). This is symmetric with creation.

### 4. Session from dirty changes

**Workflow.** The sequence for creating a session from dirty changes:

1. Resolve the current session's worktree path.
2. Check for uncommitted changes (`git status --porcelain` in that worktree). Abort if clean.
3. `git stash push` in the current worktree (captures staged + unstaged).
4. Record the current commit: `git -C <worktree> rev-parse HEAD`.
5. Create a new session via the normal flow, but override the start point so the git-worktree plugin creates the new worktree at that exact commit (not origin/main).
6. `git stash pop` in the new worktree (not the original — the stash is repo-global).

**Start-point override.** Add an optional `start_point: Option<String>` field to `SessionInfo` in the plugin protocol. When present, the git-worktree plugin uses it instead of calling `resolve_start_point()`. This is a clean protocol extension — the plugin already receives `SessionInfo`, and the field is `Option` so existing requests are unaffected.

Why a protocol field and not an env var or plugin_state: the start point is session-creation context, not persistent state. Passing it in `SessionInfo` keeps it explicit and typed. `plugin_state` is for data that survives across hook invocations; a start point is one-shot.

**Stash pop location.** The stash is applied in the *new* worktree, not the old one. Since git stash is repo-global (shared across worktrees), `git -C <new_worktree> stash pop` works. If the pop fails (conflicts), the session is still created but the user sees an error message and the stash is preserved.

**Implementation location.** This is core `ez` logic in `session/mod.rs`, not plugin logic. The session module orchestrates the stash → create → pop sequence. The git-worktree plugin only handles the worktree creation (with the overridden start point).

**CLI.** `ez session from-dirty <name>` — requires a current session context (detected via `session/current.rs`). Errors if no current session or if the current session has no worktree path.

**Browser.** A new keybind (default `alt-s`) in the session view, only active when there's a current session with dirty changes. Add `session_from_dirty: String` to `KeybindsConfig`.

**Edge cases:**
- If the current session is bare or non-git: error, stash requires a git worktree.
- If `git stash push` produces no stash (only untracked files with default stash behavior): use `git stash push --include-untracked` to capture everything.
- If the user cancels the name-builder mid-flow: pop the stash back in the original worktree to restore state.

---

## Risks / Trade-offs

**LRU timestamp write frequency.** Every `session enter` now writes `meta.toml` and `sessions.toml`. This is one extra TOML serialize+write per enter — negligible for a CLI tool, but worth noting. No locking concerns since ez is single-process.

**Bare session hook skip vs. protocol flag.** Skipping `OnSessionCreate` entirely is simple but means *all* plugins are skipped, not just git-worktree. If a future plugin (e.g., tmux) wants to act on bare sessions, we'll need to either: (a) add a separate hook, or (b) switch to the protocol-flag approach. Acceptable for now — tmux plugin binds already run independently of session creation.

**Session-from-dirty stash safety.** If ez crashes between `git stash push` and `git stash pop`, the user's changes are in the stash but not in either worktree. Mitigation: log the stash ref and print a recovery hint on error. The stash is never dropped — only popped — so data loss requires explicit user action.

**Non-git repo scope creep.** Non-git repos can't use any git-dependent features (worktrees, branches, owner view, GitHubPr name mode, session-from-dirty). Each feature that touches git needs a guard. The `is_git` field on `RepoInfo` makes these guards straightforward, but they must be added consistently across the codebase.

**Protocol backward compat.** Adding `start_point` to `SessionInfo` and `is_git` to `RepoInfo` are additive changes (new optional fields). Existing plugins that don't read these fields are unaffected. The git-worktree plugin is bundled and updated atomically with ez, so there's no version-skew risk for the primary consumer.
