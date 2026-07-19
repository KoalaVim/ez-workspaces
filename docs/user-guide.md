# User Guide

## Installation

```bash
# From git URL
cargo install --locked --git https://github.com/user/ez-workspaces.git

# From a cloned repo
git clone https://github.com/user/ez-workspaces.git
cd ez-workspaces
cargo install --locked --path .
```

### Shell Integration

Add to your shell RC file:

```bash
# ~/.zshrc or ~/.bashrc
eval "$(ez init-shell zsh)"
```

```fish
# ~/.config/fish/config.fish
ez init-shell fish | source
```

This creates a shell wrapper that enables `cd`-on-enter when you select a session and runs post-exit commands from plugins (e.g., tmux attach).

### Shell Completions

```bash
# Zsh
ez completions zsh > ~/.zfunc/_ez

# Bash
eval "$(ez completions bash)"

# Fish
ez completions fish > ~/.config/fish/completions/ez.fish
```

## First Steps

### 1. Configure workspace roots

Run the interactive setup:

```bash
ez config
```

This guides you through workspace roots, shell, selector, plugins, and timeout.

Or configure individually:

```bash
ez config add-root ~/workspace/personal
ez config add-root ~/workspace/work
ez config set default_shell zsh
```

### 2. Register repos

```bash
# Register current directory
ez add

# Register a specific path (git repo or plain directory)
ez add ~/workspace/personal/my-project

# Clone and register
ez clone git@github.com:user/repo.git
```

`ez add` works on non-git directories too — sessions become directory bookmarks without worktree management.

### 3. Enable plugins

Built-in plugins are bundled in the binary and auto-extracted on first use:

```bash
ez plugin enable git-worktree
ez plugin enable tmux

# Cursor IDE integration (enable all three for full support)
ez plugin enable cursor-mcp-auth           # Share MCP OAuth tokens across worktrees
ez plugin enable cursor-trusted-workspace  # Auto-trust worktree workspaces
ez plugin enable cursor-mcp-approvals      # Auto-approve MCP servers in worktrees
```

### 4. Create and use sessions

```bash
# Create a session (with git-worktree plugin, this creates a worktree)
# New sessions are created as children of the default (main) session unless --parent is specified
ez session new feature-login

# Create a child session under a specific parent
ez session new api-tests --parent feature-login

# Force the interactive name builder even when passing a name
ez session new my-name --interactive

# List sessions (tree view with box-drawing connectors)
ez session list
# main *
# ├── feature-login
# │   └── api-tests

# Register an existing worktree as a session (defaults to current directory and branch name)
# Also defaults under main unless --parent is specified
ez session register /path/to/worktree

# Enter a session (cd's to worktree by default; see on_enter below)
ez session enter feature-login

# Delete a named session (cascades with --force)
ez session delete feature-login --force

# Delete the current session (detects tmux @ez_session_name or current worktree, then prompts)
ez session delete
```

If `ez session new <name>` finds that the branch is already checked out in another git worktree, the git-worktree plugin reports the existing path and suggests `ez session register <path> --name <name>`. Registered worktrees are treated as session worktrees, so deleting that session runs the normal worktree cleanup hook.

#### Interactive session naming

When you create a new session *without* passing a name (`ez session new` with no
arg, or `Alt-n` in the browser), ez first presents a **mode picker** (unless
only one mode is configured):

| Mode | Description |
|------|-------------|
| **Full name** | Type the entire session name directly |
| **Build from parts** | Step through configured stages (prefix → ticket → name) |
| **From GitHub PR** | Paste a GitHub PR URL — extracts `pr<number>` and optionally resolves the branch name via the `OnNameResolve` plugin hook |
| **From Jira URL** | Paste a Jira URL — extracts `PROJ-123` then prompts for an optional suffix |

Use `--interactive` / `-i` to force the mode picker even when a name is provided
on the CLI: `ez session new my-name --interactive`.

Configure which modes are available:

```toml
name_builder_modes = ["full_name", "build_from_parts", "github_pr", "jira_url"]
```

##### Build from parts mode

In "Build from parts" mode, ez walks you through a short staged prompt and
joins the parts with `-`:

Stages come in two kinds:

- **`choice` (default)** — fzf list with the configured choices plus a
  `(none)` row. You can pick a choice, type a custom value and Enter to use
  it (when the typed text doesn't match any item), or pick `(none)` to skip
  the part.
- **`text`** — skips the fzf list and goes straight to a text prompt. Empty
  input is treated like `(none)` (the part is skipped).

Once you've picked at least one part, each subsequent stage shows the
name-so-far (e.g. `feat-ABC-`) as a header above the keybind hints so you can
see the name taking shape as you go.

`Ctrl-P` goes back to the previous stage in either kind; `Esc` cancels. The
final descriptive-name stage is implicit (always added), text-mode, and
cannot be empty.

`(none)` parts contribute nothing to the joined name. Default stages produce
names like `feat-PROJ-123-add-login-button`:

```toml
# in ~/.config/ez/config.toml — these are the defaults
[[session_name_stages]]
name = "prefix"
kind = "choice"
choices = ["feat", "fix", "chore"]

[[session_name_stages]]
name = "ticket-prefix"
kind = "choice"
choices = []  # add e.g. ["JIRA", "PROJ"]; empty just shows (none) — type your prefix and Enter

[[session_name_stages]]
name = "ticket-number"
kind = "text"  # skips fzf, prompts for free text directly
```

> Stage order is the order of `[[session_name_stages]]` blocks in the file.
> Move a block up or down to reorder the prompts. `kind` defaults to
> `"choice"` if omitted.

Passing a name on the CLI (`ez session new my-name`) skips the staged prompt
entirely. The default `main` session is also unaffected — it's always named
`main`.

#### Branch-name collision prompt

When the session name (however it was determined) matches an **existing local git
branch**, ez pauses and asks how you want to proceed:

```
Branch 'feature-login' already exists.
  [N] use the existing branch  (default)
  [y] recreate from the latest base (origin/main or parent) — discards 'feature-login'
Recreate? [y/N]:
```

- **Press Enter (or N)** — the existing branch is checked out into the new worktree as-is.
  Its commits, stashes, and history are preserved.
- **Type `y`** — the branch is deleted and re-created from the latest base (same start
  point the git-worktree plugin would use for a brand-new branch: `origin/main`,
  `origin/master`, or the parent session's HEAD). All previous commits on that branch
  are discarded.

In a non-interactive context (e.g. piped stdin), the prompt receives EOF and defaults
to **reuse**, so existing scripts keep working without modification.

### Bare sessions

Create a session without a worktree:

```bash
ez session new placeholder --bare
```

Or press **Alt-Shift-N** in the browser. Bare sessions are useful as bookmarks or placeholders — they appear in the tree with a `[bare]` indicator. The git-worktree plugin skips worktree creation for bare sessions.

### Session from dirty changes

Move uncommitted changes from the current session to a new one:

```bash
ez session from-dirty new-feature
```

Or press **Alt-s** in the browser. This stashes your uncommitted changes, creates a new session on the same commit, and applies the stash in the new session's worktree.

### 5. Browse interactively

Run bare `ez` to get an fzf-powered browser:

1. Select a workspace root
2. Drill into directories (repos show `[branch]`)
3. Select a repo to see its sessions
4. Select a session to enter it

At any top-level selector, press a keybind to switch views:

- **Ctrl-t** — Tree view: all workspaces → repos → sessions in one tree
- **Ctrl-w** — Workspace view (default): root → drill → session picker
- **Ctrl-e** — Repo view: flat list of every registered repo
- **Ctrl-o** — Owner view: repos grouped by GitHub-style owner (parsed from remote URL)
- **Ctrl-g** — Label view: items grouped by user-defined labels
- **Ctrl-a** — Tmux view (plugin): ez-managed tmux sessions — select to attach/switch

Plugin views appear automatically when enabled plugins register them. The tmux plugin adds `Ctrl-a`.

### Jumping back to a session's worktree from tmux

When the tmux plugin creates a session it stamps the session's worktree path on the tmux session as the `@ez_session_path` user option. From any pane inside that tmux session you can return to the worktree with:

```bash
ez cd-to-session
```

This reads `@ez_session_path` from the current tmux session and `cd`s your shell to it (via the shell wrapper installed by `ez init-shell`). Useful after navigating elsewhere or when opening a new pane that didn't inherit the cwd.

> The option is only set when the tmux session is created by the plugin (on `session new`, the `Ctrl-a` bind, or the tmux view). Pre-existing sessions created before this feature won't have it — recreate them or trigger `Ctrl-a` from the picker to stamp it.

Inside the session picker:

- **Alt-n** — New child session
- **Alt-Shift-N** — New bare session (no worktree)
- **Alt-s** — Session from dirty (move uncommitted changes to new session)
- **Alt-r** — Rename session
- **Alt-d** — Delete session
- **Alt-l** — Edit labels (comma-separated, prefix `-` to remove, e.g. `wip, -stale`)
- **Ctrl-d** — Cd into session worktree (bypasses on_enter action like tmux)
- **Ctrl-s** — Toggle sort (alphabetical / LRU)

You can also launch a specific view directly: `ez --select-by repo`, `ez --select-by label`, etc. To change the default view, set `default_select_by = "repo"` in your config (or run `ez config set default_select_by repo`).

Repos and sessions are sorted by last-accessed time (LRU) by default. Press **Ctrl-s** to toggle between LRU and alphabetical sort. To change the default:

```toml
default_sort = "lru"    # lru | alpha
```

### Configuring what Enter does (`on_enter`)

By default, pressing **Enter** on a session (or running `ez session enter <name>`) **cd's into the session's worktree**. You can change this to any session plugin-bind by name:

```bash
# Attach to (or create) the tmux session instead of cd-ing
ez config set on_enter tmux

# Override per-invocation (overrides config)
ez --on-enter tmux
ez --on-enter cd          # force cd even if config says tmux
```

`on_enter` is matched against a session plugin-bind's **label**, **bind name**, or **plugin name** — so `"tmux"` resolves to the tmux plugin's `tmux_attach` bind (the same action as pressing **Alt-a** in the picker). If the named bind is unavailable (plugin disabled, tmux not installed), ez silently falls back to `cd`.

Set it in `~/.config/ez/config.toml`:

```toml
on_enter = "tmux"    # cd | tmux (or any session plugin-bind label/name)
on_create = "tmux"   # none | cd | tmux (or any session plugin-bind label/name)
```

### Configuring what happens after creating a session (`on_create`)

By default, creating a session (picker **Alt-n** or `ez session new <name>`) just creates it and does nothing else. You can make it immediately jump in:

```bash
# After creating a session, cd into its worktree
ez config set on_create cd

# After creating a session, attach to (or create) its tmux session
ez config set on_create tmux

# Per-invocation override
ez --on-create tmux session new my-feature
```

In the interactive picker, when `on_create` is set, **Alt-n** creates the session, performs the action, and exits (just like pressing Enter on an existing session). With `"none"` (default) it stays in the picker as it does today.

If the named bind is unavailable (plugin disabled, tmux not installed), ez silently falls back to `cd`.

## Labels

Tag any repo or session to group and filter them.

```bash
# Add labels
ez repo label add my-repo backend core
ez session label add feature-x --repo my-repo wip

# Remove labels
ez repo label remove my-repo core

# List
ez repo label list              # all labels grouped
ez repo label list my-repo      # labels on one repo
ez repo list --label backend    # filter repo list

# Browse by label
ez --select-by label
```

Labels on the currently selected item can also be edited interactively in the browser by pressing **Alt-l**. Labels are stored in the repo's metadata (`~/.config/ez/repos/<id>/repo.toml`) and in per-session metadata (`sessions.toml`).

## Session Hierarchy

Sessions form a tree. New sessions are created as children of the default (main) session unless `--parent` is specified. Use `--parent` to nest under a different session:

```
main *                    # auto-created default
├── feature-auth          # child of main (default)
│   ├── backend-api       # child of feature-auth
│   └── frontend-ui       # child of feature-auth
└── bugfix-crash          # child of main (default)
```

The default "main" session is auto-created when you first access a repo. It points to the repo's working directory. Box-drawing connectors (tree glyphs) show parent-child relationships in `ez session list` and in the session picker.

### Return-to-ez after tmux detach

When `on_enter` is set to `tmux`, the shell wrapper automatically re-enters the ez browser after detaching from a tmux session (`Ctrl-b d` or `tmux detach`). This creates a seamless workflow loop: browse → attach → work → detach → browse again. No additional config needed.

## Non-git Sessions

Sessions work without git. Without the git-worktree plugin, sessions are purely virtual — just metadata with a name, parent relationships, and environment variables. This is useful for organizing work contexts even in non-git projects.

## Colored Output

All output is colored by default. To disable:

```bash
ez --no-color session list
```

Or set the `NO_COLOR` environment variable (respected automatically).

## Escape / Back Navigation

In interactive menus (browsing directories, config wizard), pressing **Escape** goes back to the previous level instead of quitting. At the top level, Escape exits.

## Environment Variables

Sessions can carry environment variables (set by plugins). When you enter a session, these are available in your shell.
