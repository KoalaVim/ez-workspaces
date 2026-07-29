## MODIFIED Requirements

### Requirement: Post-command-file pattern
The system SHALL write post-exit shell commands to the post-cmd-file (passed via `--post-cmd-file`). These are commands that must run in the user's shell after ez exits, such as `tmux switch-client` or `tmux kill-session`. If no post-cmd-file is available, the system warns about an outdated shell wrapper and runs the commands inline as a fallback.

#### Scenario: Post-exit tmux kill on session delete
- **WHEN** user deletes a session that has an associated tmux session
- **THEN** system writes `tmux kill-session -t "=<tmux-session-name>"` to the post-cmd-file
- **THEN** the shell wrapper sources the file after ez exits, killing the tmux session

#### Scenario: Delete without shell wrapper
- **WHEN** `--post-cmd-file` is not provided and session delete needs to kill a tmux session
- **THEN** system runs `tmux kill-session` inline as a fallback (same as current reaper behavior)
