---
name: "ez-browse"
description: "Interactive workspace browser using fzf. Use when working on the browser UI, selector trait, preview rendering, or fzf integration."
---

# ez-workspaces Interactive Browser

## What This Skill Does

Covers the interactive fzf-based browser that lets users drill into workspace directories, find repos, and select sessions. This is the main UX of `ez` (bare command with no args).

## Key Files

- `src/browser/mod.rs` — Browse orchestration, drill-down flow, preview handler
- `src/browser/selector.rs` — `InteractiveSelector` trait + `FzfSelector` implementation

## Browse Flow

1. Show workspace roots from `config.workspace_roots`
2. User selects a root → show subdirectories
3. Repos annotated with `[branch]`, directories with `/` suffix
4. fzf `--preview` calls `ez _preview <path>` for live preview
5. Drill into directories until a git repo is selected
6. Auto-register repo if not yet registered
7. Auto-create "main" session if none exist
8. Show session tree → user selects → enter session

## InteractiveSelector Trait

```rust
pub trait InteractiveSelector {
    fn select_one(&self, items: &[SelectItem], prompt: &str, preview_cmd: Option<&str>) -> Result<Option<usize>>;
    fn input(&self, prompt: &str, default: Option<&str>) -> Result<String>;
    fn confirm(&self, prompt: &str, default: bool) -> Result<bool>;
}
```

- `FzfSelector`: shells out to fzf with `--preview`, `--layout reverse`, `--ansi`
- Returns `None` on Escape/Ctrl-C (fzf exit code 130)
- Extra fzf flags configurable via `config.selector.fzf_opts`

## Preview

The hidden `ez _preview <path>` command renders:
- **Directory**: lists subdirectories, repos with `[branch]`
- **Registered repo**: shows session tree
- **Unregistered repo**: shows branch name

## Testing

Use a `MockSelector` implementing `InteractiveSelector` to test browse flow without fzf:

```rust
struct MockSelector { selections: Vec<usize> }
impl InteractiveSelector for MockSelector {
    fn select_one(&self, ...) -> Result<Option<usize>> {
        Ok(Some(self.selections[self.call_count]))
    }
}
```

## Configuration

```toml
[selector]
backend = "fzf"
fzf_opts = "--border --info=inline"
```
