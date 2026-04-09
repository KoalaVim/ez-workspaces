# ez-workspaces

A fast, plugin-extensible workspace and session manager for git repos. Browse, create, and switch between worktree-based sessions with a single command.

## Installation

```bash
# From git URL
cargo install --locked --git https://github.com/user/ez-workspaces.git

# From a cloned repo
git clone https://github.com/user/ez-workspaces.git
cd ez-workspaces
cargo install --locked --path .
```

## Quick Start

```bash
# Add shell integration (add to your .zshrc/.bashrc)
eval "$(ez init-shell zsh)"

# Register a repo
ez add ~/my-project

# Create sessions
ez session new feature-auth
ez session new sub-task --parent feature-auth

# List sessions (tree view)
ez session list
# feature-auth
#   sub-task

# Enter a session
ez session enter feature-auth

# Interactive browser (the killer feature)
ez
```

## How It Works

**ez** treats git worktrees as "sessions" organized in a tree hierarchy. Each session is virtual metadata — plugins give sessions physical meaning:

- **git-worktree plugin**: creates/deletes worktrees on session create/delete
- **tmux plugin**: creates/attaches tmux sessions on enter

Sessions are tree-based — a session can have child sessions, enabling branching workflows.

## Commands

| Command | Description |
|---------|-------------|
| `ez` | Interactive fzf browser |
| `ez clone <url> [path]` | Clone + register repo |
| `ez add [path]` | Register existing repo |
| `ez session new [name]` | Create session (`--parent` for nesting) |
| `ez session list` | List sessions as tree (`--flat` for flat) |
| `ez session enter <name>` | Enter a session |
| `ez session delete <name>` | Delete session (`--force` for cascade) |
| `ez session rename <old> <new>` | Rename a session |
| `ez repo list` | List registered repos |
| `ez repo remove <name>` | Unregister repo (`--purge` for cleanup) |
| `ez plugin list` | List available plugins |
| `ez plugin enable <name>` | Enable a plugin |
| `ez config` | Interactive guided setup |
| `ez config show` | Show current config |
| `ez config edit` | Open config in editor |
| `ez config add-root <path>` | Add a workspace root |
| `ez config set <key> <value>` | Set a config value |
| `ez config get <key>` | Get a config value |
| `ez init-shell <shell>` | Print shell wrapper function |
| `ez completions <shell>` | Generate shell completions |

## Configuration

Run the interactive setup:

```bash
ez config
```

This walks through workspace roots, shell, selector backend, plugins, and timeout.

Or configure individually:

```bash
ez config add-root ~/workspace/personal
ez config set selector.backend fzf
ez config set default_shell zsh
```

Config file: `~/.config/ez/config.toml`

```toml
workspace_roots = ["~/workspace/personal", "~/workspace/work"]
default_shell = "zsh"
plugin_timeout = 30

[selector]
backend = "fzf"

[plugins]
enabled = ["git-worktree", "tmux"]
```

## Shell Completions

```bash
# Zsh
ez completions zsh > ~/.zfunc/_ez

# Bash
eval "$(ez completions bash)"

# Fish
ez completions fish > ~/.config/fish/completions/ez.fish
```

## Plugins

Plugins are external scripts in `~/.config/ez/plugins/<name>/`. Copy the bundled plugins from `plugins/` to get started:

```bash
cp -r plugins/* ~/.config/ez/plugins/
ez plugin enable git-worktree
ez plugin enable tmux
```

See [Plugin Guide](docs/plugin-guide.md) for writing custom plugins.

## Docs

- [User Guide](docs/user-guide.md)
- [Plugin Guide](docs/plugin-guide.md)
- [Architecture](docs/architecture.md)
- [Design (Mermaid diagrams)](docs/design.md)
- [Claude Code Skills](docs/skills.md)

## Requirements

- Rust 1.70+
- [fzf](https://github.com/junegunn/fzf) (for interactive mode)
- git (for worktree plugin)
- jq (for bundled plugins)
- tmux (optional, for tmux plugin)

## License

AGPL-3.0
