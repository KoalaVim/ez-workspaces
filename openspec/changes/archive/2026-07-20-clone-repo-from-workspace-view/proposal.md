## Why

When browsing the workspace view and drilling into directories, users often discover repos they want to clone but must leave the browser to run `ez clone <url>` in a terminal. Adding a clone action directly in the workspace drill-down view eliminates this context switch — the user can clone a new repo right where they'd expect to find it, then immediately enter its session picker.

## What Changes

- Add a keybind (e.g. `alt-a` for "add/clone") in the workspace drill-down directory browser that prompts for a git URL and clones the repo into the current browsed directory.
- After a successful clone, auto-register the repo and transition directly into its session picker.
- Reuse the existing `repo::clone_repo` logic but allow overriding the target directory to the currently browsed workspace path.
- Add the new keybind to the `KeybindsConfig` so users can customize it.
- Show the clone keybind in the directory preview pane's keybind help.

## Capabilities

### New Capabilities
- `clone-from-browser`: Ability to clone a git repo directly from the workspace drill-down view via a keybind, prompting for a URL and cloning into the browsed directory.

### Modified Capabilities
- `interactive-browser`: Add the clone keybind to the workspace drill-down directory browser and its preview keybind help.
- `repo-management`: No spec-level requirement changes — existing `clone_repo` is reused as-is.

## Impact

- `src/browser/mod.rs`: `drill_into_directory` gains a new keybind action and clone flow.
- `src/config/model.rs`: New `clone_repo` keybind field in `KeybindsConfig`.
- `src/browser/preview.rs`: Add clone keybind to directory preview help table.
- `docs/user-guide.md`, `README.md`, `AGENTS.md`: Document the new keybind.
