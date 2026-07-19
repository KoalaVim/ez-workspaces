## Why

The current README is comprehensive but too long — it reads like a reference manual rather than a landing page. Developers evaluating the project need to quickly understand what it does, why it exists, and how to get started. Detailed command tables and config examples belong in docs, not the README.

## What Changes

- Rewrite README to be concise and scannable (~100 lines max)
- Lead with a one-liner + demo GIF placeholder
- Add a "Why" section explaining the core pain point: fast context switching between tasks via worktree-backed sessions
- Streamline Quick Start to 5-6 commands
- Replace full command table with a grouped overview of key capabilities
- Show browser keybinds as a compact section, not exhaustive tables
- Move detailed command reference, config options, and name builder docs to user-guide.md
- Keep plugin overview but trim to essentials
- Link to full docs for everything else

## Capabilities

### New Capabilities

(none — this is a documentation-only change)

### Modified Capabilities

(none — no spec-level behavior changes, only README restructuring)

## Impact

- `README.md`: full rewrite
- `docs/user-guide.md`: may receive content moved from README (command table, config details)
- No code changes
