# User Guide

## Installation

```bash
cargo build --release
cp target/release/ez ~/.local/bin/
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

## First Steps

### 1. Configure workspace roots

```bash
ez config --edit
```

Add your workspace directories:

```toml
workspace_roots = ["~/workspace/personal", "~/workspace/work"]
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

### 3. Install plugins

```bash
# Copy bundled plugins
cp -r /path/to/ez-workspaces/plugins/* ~/.config/ez/plugins/
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

## Environment Variables

Sessions can carry environment variables (set by plugins). When you enter a session, these are available in your shell.
