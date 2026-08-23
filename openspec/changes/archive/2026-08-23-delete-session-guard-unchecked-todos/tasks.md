## 1. Core Detection Logic

- [x] 1.1 Add `has_unchecked_todos(repo_id, session_id) -> Vec<String>` to `src/session/notes.rs` that reads README.md and returns lines matching `^\s*- \[ \]`
- [x] 1.2 Add `SessionHasUncheckedTodos` error variant to `src/error.rs` with session names and todo lines

## 2. Wire into CLI Delete Flow

- [x] 2.1 Add unchecked-todos pre-flight check in `delete_session` (after dirty worktree check, before removal), collecting todos from all sessions in `to_reap`
- [x] 2.2 Add the same guard in `delete_session_by_id` for the browser delete path

## 3. Wire into Browser Delete

- [x] 3.1 Add `cascade_unchecked_todos(repo_id, session_id) -> Vec<String>` public function (mirrors `cascade_dirty`) for the browser to show warnings
- [x] 3.2 Display unchecked-todos warning in the browser delete confirmation (same pattern as dirty worktree warning)

## 4. Verify

- [x] 4.1 Manual test: create session with unchecked todo in notes, verify delete is blocked
- [x] 4.2 Manual test: verify `--force` bypasses the guard
- [x] 4.3 Manual test: verify cascade delete reports descendant session todos
