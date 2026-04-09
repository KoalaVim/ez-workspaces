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

This creates a shell wrapper that enables `cd`-on-enter when you select a session.

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

# Enter a session (cd's to worktree, attaches tmux)
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
