## MODIFIED Requirements

### Requirement: Hook response mutations
The `HookResponse` SHALL support: `session_mutations` (modify session fields like path, env, plugin_state), `repo_mutations` (modify repo metadata fields), `shell_commands` (run inline during ez execution), `post_shell_commands` (written to post-cmd-file, sourced by shell wrapper after ez exits), `cd_target` (path to cd into), `keep_cwd` (bool, default false — the plugin handled navigation itself, so ez SHALL NOT cd the invoking shell), and `view_items` (list of items for `OnView`).

#### Scenario: Session path mutation
- **WHEN** git-worktree plugin responds to `OnSessionCreate`
- **THEN** response includes `session_mutations` with `path` set to the new worktree path

#### Scenario: Post-shell commands
- **WHEN** tmux plugin responds to `OnBind`
- **THEN** response includes `post_shell_commands` like `tmux switch-client -t session-name`

#### Scenario: Keep cwd
- **WHEN** a plugin responds with `keep_cwd: true`
- **THEN** ez writes no cd target of its own for that response
- **AND** the shell that invoked ez stays in its original working directory

#### Scenario: Keep cwd omitted
- **WHEN** a plugin response omits `keep_cwd`
- **THEN** it is treated as `false` and existing cd behavior is unchanged
