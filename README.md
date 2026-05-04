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
- **tmux plugin**: creates tmux sessions, attach via `Ctrl-a` view or auto-attach on enter

Sessions are tree-based — a session can have child sessions, enabling branching workflows.

### Where worktrees live

The git-worktree plugin creates worktrees as siblings of the repo in a `.ez-worktrees` directory:

```
~/workspace/personal/
  my-repo/                          # original repo
  .ez-worktrees/my-repo/
    feature-auth/                   # worktree for session "feature-auth"
    bugfix-crash/                   # worktree for session "bugfix-crash"
```

Each worktree gets its own branch (`ez/<session-name>`) branched from HEAD at creation time.

## Commands

| Command | Description |
|---------|-------------|
| `ez` | Interactive fzf browser (default view is `default_select_by` in config, else `workspace`) |
| `ez --select-by <mode>` | Start the browser by `tree`, `workspace`, `repo`, `owner`, or `label` |
| `ez --workspace <name>` | Jump directly to a workspace root |
| `ez --repo <path>` | Jump straight to a repo's session picker |
| `ez clone <url> [path]` | Clone + register repo |
| `ez add [path]` | Register existing repo |
| `ez session new [name]` | Create session (`--parent` for nesting) |
| `ez session list` | List sessions as tree (`--flat` for flat) |
| `ez session enter <name>` | Enter a session |
| `ez session delete <name>` | Delete session (`--force` for cascade) |
| `ez session rename <old> <new>` | Rename a session |
| `ez cd-to-session` | From inside a tmux session, cd back to that ez session's worktree (reads the `@ez_session_path` tmux option set by the tmux plugin) |
| `ez session label add <name> <label>...` | Add labels to a session |
| `ez session label remove <name> <label>...` | Remove labels from a session |
| `ez session label list [<name>]` | List labels (or group sessions by label) |
| `ez repo list [--label <label>]` | List registered repos (optionally filter by label) |
| `ez repo remove <name>` | Unregister repo (`--purge` for cleanup) |
| `ez repo label add <name> <label>...` | Add labels to a repo |
| `ez repo label remove <name> <label>...` | Remove labels from a repo |
| `ez repo label list [<name>]` | List labels (or group repos by label) |
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
| `ez --no-color <command>` | Disable colored output |

## Browser Views & Keybinds

Inside the interactive browser (`ez`), press a keybind at the top-level selector to switch views:

| Keybind (default) | View | What it shows |
|---|---|---|
| `Ctrl-t` | Tree | All workspaces → repos → sessions in one tree |
| `Ctrl-w` | Workspace | Workspace root → drill into directories → session picker (default view) |
| `Ctrl-e` | Repo | Flat list of every registered repo |
| `Ctrl-o` | Owner | Repos grouped by GitHub-style owner (parsed from remote URL) |
| `Ctrl-g` | Label | Items grouped by user-defined labels |
| `Ctrl-a` | Tmux (plugin) | Ez-managed tmux sessions (requires tmux plugin) |

Plugin views appear automatically when a plugin is enabled. The tmux plugin registers `Ctrl-a` by default.

Inside the session picker (and the flat Repo view):

| Keybind (default) | Action |
|---|---|
| `Alt-n` | New child session |
| `Alt-r` | Rename session |
| `Alt-d` | Delete session |
| `Alt-l` | Edit labels on the selected item (comma-separated, prefix `-` to remove) |

All keybinds are configurable under `[keybinds]` in `~/.config/ez/config.toml`:

```toml
[keybinds]
new_session = "alt-n"
delete_session = "alt-d"
rename_session = "alt-r"
view_tree = "ctrl-t"
view_workspace = "ctrl-w"
view_repo = "ctrl-e"
view_owner = "ctrl-o"
view_label = "ctrl-g"
edit_labels = "alt-l"
```

## Labels

Repos and sessions can be tagged with arbitrary string labels. Labels aggregate across all registered repos in the Label view and let you filter repo listings.

```bash
# Tag a repo and a session
ez repo label add my-repo backend core
ez session label add feature-x --repo my-repo wip

# Filter the list
ez repo list --label backend

# Browse by label (or press Ctrl-g inside `ez`)
ez --select-by label
```

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
default_select_by = "workspace"  # tree | workspace | repo | owner | label

[selector]
backend = "fzf"

[plugins]
enabled = ["git-worktree", "tmux"]

[plugin_settings.tmux]
auto_attach = true  # auto-attach to tmux session on session enter
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

Built-in plugins (git-worktree, tmux) are bundled in the binary and auto-extracted on first use. Just enable them:

```bash
ez plugin enable git-worktree
ez plugin enable tmux
```

Plugins can register custom views (shown as extra keybinds in the browser), declare configuration options, and run commands in the user's shell after ez exits. See [Plugin Guide](docs/plugin-guide.md).

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
