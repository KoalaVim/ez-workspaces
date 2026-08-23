## Why

The session list in the interactive browser shows `→ <path>` after each session name, adding visual noise without much value — the path is already visible in the preview pane. Additionally, there's no visual indicator for which sessions are currently attached (open in a tmux, herdr, or zellij pane), making it hard to tell at a glance which sessions are active.

## What Changes

- Remove the `→ <path>` suffix from session entries in the interactive browser list and `ez session list` CLI output.
- Detect attached sessions (those currently open in tmux, herdr, or zellij) and render their name in aqua/cyan color instead of the default yellow.

## Capabilities

### New Capabilities

- `attached-session-indicator`: Detect which sessions are currently attached via a terminal multiplexer (tmux, herdr, zellij) and visually distinguish them in the session list by rendering their name in aqua.

### Modified Capabilities

- `session-management`: Remove `→ <path>` display from session list output (both interactive browser and CLI `session list`).

## Impact

- `src/browser/mod.rs` — session item rendering in the fzf selector (line ~385) and `ez session list` display
- `src/session/mod.rs` — CLI `session list` output (line ~707-741)
- `src/session/current.rs` — may need to expose multiplexer-detection logic for reuse
- `src/browser/preview.rs` — session preview rendering (lines ~273, ~374)
- `src/browser/views/tree.rs` — tree view session rendering (line ~115)
- Multiplexer plugins (tmux, herdr, zellij) — may need to query for attached session state
