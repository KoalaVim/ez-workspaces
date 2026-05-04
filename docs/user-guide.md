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

# Register a specific path
ez add ~/workspace/personal/my-project

# Clone and register
ez clone git@github.com:user/repo.git
```

### 3. Enable plugins

Built-in plugins are bundled in the binary and auto-extracted on first use:

```bash
ez plugin enable git-worktree
ez plugin enable tmux
```

### 4. Create and use sessions

```bash
# Create a session (with git-worktree plugin, this creates a worktree)
ez session new feature-login

# Create a child session
ez session new api-tests --parent feature-login

# List sessions
ez session list
# feature-login
#   api-tests

# Enter a session (cd's to worktree; attaches tmux if auto_attach is on)
ez session enter feature-login

# Delete (cascades with --force)
ez session delete feature-login --force
```

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
- **Alt-r** — Rename session
- **Alt-d** — Delete session
- **Alt-l** — Edit labels (comma-separated, prefix `-` to remove, e.g. `wip, -stale`)

You can also launch a specific view directly: `ez --select-by repo`, `ez --select-by label`, etc. To change the default view, set `default_select_by = "repo"` in your config (or run `ez config set default_select_by repo`).

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

Sessions form a tree. Use `--parent` to nest sessions:

```
main *                    # auto-created default
feature-auth              # root-level session
  backend-api             # child of feature-auth  
  frontend-ui             # child of feature-auth
bugfix-crash              # another root-level session
```

The default "main" session is auto-created when you first access a repo. It points to the repo's working directory.

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
