## Context

The current README.md is ~300 lines covering every command, config option, keybind, and name builder mode. It serves as a reference manual but fails as a project landing page. Developers evaluating the tool spend too long scrolling before understanding what it does. The user guide already exists but duplicates much of the README content.

## Goals / Non-Goals

**Goals:**
- README fits in ~100 lines, readable in under 2 minutes
- Clear "why this exists" section targeting developers who context-switch frequently
- 5-command quick start that gets someone productive immediately
- Demo GIF placeholder at the top for visual first impression
- Browser and plugin sections that convey power without exhaustive tables
- Full reference content lives in docs/user-guide.md

**Non-Goals:**
- Rewriting user-guide.md (only move content there if needed)
- Changing any code or behavior
- Creating marketing copy — keep it technical and honest

## Decisions

### README structure

```
1. Title + one-liner tagline
2. Demo GIF placeholder
3. Why (2-3 sentences on the pain point)
4. Install (cargo install)
5. Quick Start (5-6 commands: init-shell, enable plugins, add repo, create session, launch browser)
6. Browser (paragraph + compact keybind summary, not full tables)
7. Plugins (list bundled plugins in one block, mention Cursor plugins)
8. Docs links
9. Requirements + License
```

**Rationale**: Front-load the "what" and "why", then setup, then features. Detailed reference stays in user-guide.md.

### Content to move vs. remove

- Full command table → already in user-guide.md, remove from README
- Config TOML examples → keep minimal example in Quick Start, full config in user-guide
- Name builder modes table → remove from README (user-guide has it)
- Labels section → brief mention, details in user-guide
- Shell completions → move to user-guide
- Keybind config TOML block → remove from README

### Keybind presentation

Show browser views and session keybinds as two compact lists (not tables). Mention "all configurable" with a link.

## Risks / Trade-offs

- [Users may miss detailed info] → Mitigated by clear links to docs/user-guide.md
- [README may feel too sparse] → The GIF placeholder and concise writing should compensate; can always add back if feedback suggests
