## MODIFIED Requirements

### Requirement: Session action loop
When a repo is selected, the browser SHALL enter a session action loop that repeatedly shows the repo's sessions as a tree with box-drawing tree connectors (`├──`, `└──`, `│`) and handles keybind actions until the user selects a session (Enter) or cancels (Escape). The loop re-renders after each action to show updated state. Sessions SHALL be rendered with tree glyphs matching the indentation style used in the Tree view. The loop SHALL support additional keybinds: `Alt-Shift-N` for bare session creation, `alt-s` for session-from-dirty, `ctrl-s` for sort toggle, `alt-i` for opening the session's note in the configured editor, and `alt-I` for cd-ing to the session's notes directory.

Before rendering items, the loop SHALL build a worktree info cache by calling `git worktree list --porcelain` once. This cache SHALL be used for both per-session branch display (HashMap lookup instead of `git symbolic-ref` subprocess) and unmanaged worktree detection. The cache SHALL be rebuilt on each loop iteration to reflect any branch changes caused by actions.

After the managed session items, the picker SHALL append a "Not Registered" section showing non-managed git worktrees detected from the same worktree cache. These items SHALL appear below a non-interactive header line reading "Not Registered". Each non-managed worktree SHALL display the branch name (dimmed) and worktree path (dimmed). Selecting a non-managed worktree SHALL register it as a session under the default (main) session and enter it using the configured `on_enter` action. The "Not Registered" section SHALL only appear when there are non-managed worktrees to show.

#### Scenario: Session branches resolved from worktree cache
- **WHEN** the session action loop renders items for a repo with 11 sessions
- **THEN** the system calls `git worktree list --porcelain` once (not `git symbolic-ref` 11 times)
- **AND** each session's branch indicator is resolved via HashMap lookup from the parsed output

#### Scenario: Re-render after sort toggle uses single git call
- **WHEN** user presses `ctrl-s` to toggle sort in the session picker
- **THEN** the loop rebuilds the worktree cache with one `git worktree list` call
- **AND** does not spawn per-session `git symbolic-ref` calls

#### Scenario: Select session
- **WHEN** user presses Enter on a session
- **THEN** system runs the `on_enter` action (default: cd into session path)

#### Scenario: Cancel returns to view layer
- **WHEN** user presses Escape in the session action loop
- **THEN** system returns to the previous view level (the view that was active before repo selection)

### Requirement: Repo view
The Repo view SHALL display a flat list of all registered repos with name, path, branch, and labels. Selecting a repo SHALL transition to its session picker. The view SHALL also support session actions (new, delete, rename, labels) and view-switch keybinds.

Branch resolution for all repos SHALL be performed concurrently using `std::thread::scope`. The mtime-based branch cache SHALL be used so that re-renders after non-branch-changing actions (label edit, sort toggle) avoid subprocess calls entirely.

#### Scenario: Display all repos with parallel branch resolution
- **WHEN** Repo view is displayed with 19 registered repos
- **THEN** system resolves all 19 branches concurrently
- **AND** total branch resolution wall time is approximately the time of the single slowest git call

#### Scenario: Re-render after label edit uses cache
- **WHEN** user edits repo labels and the repo view re-renders
- **AND** no branches have changed since the last render
- **THEN** all branch lookups are cache hits (zero subprocess calls)

### Requirement: Tree view
The Tree view SHALL render all workspace roots, their repos, and each repo's sessions in a single indented tree with ASCII box-drawing connectors. Selecting a session SHALL enter it using the `accept_session` flow (which handles the configured `on_enter` action including plugin binds like tmux attach), passing the `post_cmd_file` for post-exit commands. Selecting a workspace root SHALL re-render.

Repo-level branches SHALL be resolved concurrently across repos. Per-session branches within each repo SHALL be resolved from the worktree list cache (one `git worktree list --porcelain` call per repo, not per session).

#### Scenario: Render full tree with batched branches
- **WHEN** Tree view is displayed with 2 workspace roots, 10 repos, and 50 sessions total
- **THEN** system runs `git worktree list --porcelain` once per repo (not `git symbolic-ref` per session)
- **AND** repo-level branches are resolved concurrently across repos

#### Scenario: Select session in tree
- **WHEN** user selects a session row in the tree
- **THEN** system runs the `accept_session` flow with `post_cmd_file` passthrough, applying the configured `on_enter` action (cd, tmux attach, or other plugin bind)
