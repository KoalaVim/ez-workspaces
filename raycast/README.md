# EZ Workspaces — Raycast Integration

Two options for integrating ez-workspaces with [Raycast](https://raycast.com) (macOS launcher).

## Option A: Raycast Extension (Recommended)

A full interactive extension with searchable lists, drill-down navigation, and action panels.

### Features

- **Browse Repos** — searchable list with view mode switching (Repo, Owner, Label, Workspace)
- **Session drill-down** — select a repo to browse its sessions with tree hierarchy, LRU timestamps, and PR status
- **Search Sessions** — flat cross-repo session search
- **Actions** — Enter session (opens Terminal), Open in Cursor, Show in Finder, Copy Path, Open PR, Delete session

### Installation

```bash
cd raycast/ez-extension
npm install
npm run dev    # opens in Raycast dev mode
```

After running `npm run dev`, the extension appears in Raycast search. Press `Ctrl+C` to stop dev mode — the extension stays installed.

To rebuild after changes:

```bash
npm run build
```

### Prerequisites

- `ez` must be installed and in PATH (check with `which ez`)
- Node.js and npm

## Option B: Script Commands (Lightweight Fallback)

Simple bash scripts that display text output. No npm required.

- **ez-repos.sh** — Lists all registered repos via `ez repo list --json`
- **ez-sessions.sh** — Lists sessions for a given repo via `ez session list --json --repo <name>`

### Prerequisites

- `ez` must be installed and in PATH
- `jq` must be installed (`brew install jq`)

### Installation

Symlink into Raycast's script commands directory:

```bash
ln -s "$(pwd)/raycast/ez-repos.sh" ~/.config/raycast/script-commands/ez-repos.sh
ln -s "$(pwd)/raycast/ez-sessions.sh" ~/.config/raycast/script-commands/ez-sessions.sh
```

Open Raycast → Extensions → Script Commands → Reload.
