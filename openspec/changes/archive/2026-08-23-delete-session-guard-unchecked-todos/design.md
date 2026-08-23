## Context

The session delete flow (`delete_session` and `delete_session_by_id`) already has a pre-flight guard that checks for dirty worktrees. Session notes are stored as markdown files in the data directory (`<data_dir>/ez/repos/<repo-id>/notes/<session-id>/README.md`). The existing `notes.rs` module already provides `notes_readme_exists` and path helpers.

## Goals / Non-Goals

**Goals:**
- Prevent accidental loss of tracked work by warning when notes contain unchecked todos
- Integrate seamlessly with the existing `--force` bypass and dirty-worktree pattern
- Apply to both CLI and browser delete paths

**Non-Goals:**
- Scanning files other than README.md in the notes directory
- Supporting custom todo patterns beyond the standard `- [ ]` markdown checkbox
- Making the guard configurable (disable/enable per-repo) — can be added later if needed

## Decisions

### Detection logic lives in `notes.rs`
Add a `has_unchecked_todos(repo_id, session_id) -> bool` function that reads the README.md and checks for lines matching `- [ ]` (with optional leading whitespace). This keeps all notes-related logic in one module.

**Alternative considered:** A regex-based scanner that handles nested lists, blockquotes, etc. Rejected as over-engineering — the simple line-contains check covers real usage and is trivial to understand.

### Reuse the `--force` bypass
The unchecked-todos guard uses the same `force` flag as the dirty-worktree check. No new flags or separate confirmation prompt. This keeps the UX consistent: `--force` means "I know what I'm doing, skip all guards."

**Alternative considered:** A separate `--ignore-todos` flag. Rejected because it fragments the force semantics and adds CLI surface area for a niche case.

### Error variant reports the todo items
The error message lists the unchecked todo lines so the user knows exactly what's outstanding. Capped at 5 lines to avoid flooding the terminal.

### Cascade deletes check all descendants
For cascade deletes, the guard collects unchecked todos from all sessions being deleted (parent + descendants), same pattern as `dirty_worktrees`. The error reports which sessions have outstanding todos.

## Risks / Trade-offs

- [False positives from code blocks] → Acceptable for v1. Users writing `- [ ]` in fenced code blocks will trigger the guard, but can bypass with `--force`. Can add code-block-aware parsing later if it becomes an issue.
- [Performance on many descendants] → Reading a small README.md per session is negligible I/O, especially since cascades are rare and small.
