## MODIFIED Requirements

### Requirement: Keybinds configuration
The config SHALL support a `[keybinds]` section with configurable keys for: `new_session` (default alt-n), `delete_session` (alt-d), `rename_session` (alt-r), `cd_session` (default alt-c), `view_tree` (ctrl-t), `view_workspace` (ctrl-w), `view_repo` (ctrl-e), `view_owner` (ctrl-o), `view_label` (ctrl-g), `edit_labels` (alt-l), `bare_session` (default alt-shift-n), `session_from_dirty` (default alt-s), `sort_toggle` (default ctrl-s), `note_open` (default alt-i), `note_cd` (default alt-I).

#### Scenario: Custom keybind
- **WHEN** config has `[keybinds] new_session = "alt-c"`
- **THEN** the browser uses Alt-c for creating new sessions instead of Alt-n

#### Scenario: Cd session keybind
- **WHEN** config has `[keybinds] cd_session = "alt-g"`
- **THEN** the browser uses Alt-g for the cd action instead of the default Alt-c

#### Scenario: Default cd keybind
- **WHEN** no `cd_session` keybind is configured
- **THEN** the browser uses Alt-c as the default keybind for cd-ing into a session

#### Scenario: Bare session keybind override
- **WHEN** config has `[keybinds] bare_session = "alt-b"`
- **THEN** the browser uses Alt-b for bare session creation instead of Alt-Shift-N

#### Scenario: Sort toggle keybind override
- **WHEN** config has `[keybinds] sort_toggle = "ctrl-r"`
- **THEN** the browser uses Ctrl-r for sort toggle instead of ctrl-s

#### Scenario: Session from dirty keybind override
- **WHEN** config has `[keybinds] session_from_dirty = "alt-shift-s"`
- **THEN** the browser uses Alt-Shift-S for session-from-dirty instead of alt-s

#### Scenario: Note open keybind override
- **WHEN** config has `[keybinds] note_open = "alt-o"`
- **THEN** the browser uses Alt-o for opening session notes instead of alt-i

#### Scenario: Note cd keybind override
- **WHEN** config has `[keybinds] note_cd = "alt-O"`
- **THEN** the browser uses Alt-Shift-O for cd-ing to notes directory instead of alt-I

#### Scenario: Default note keybinds
- **WHEN** no `note_open` or `note_cd` keybinds are configured
- **THEN** the browser uses alt-i for note open and alt-I for note cd

### Requirement: Note open command configuration
The config SHALL support a `note_open_command` field (default `"$EDITOR"`) that specifies the command used to open session note files. The value `"$EDITOR"` SHALL be resolved from the environment variable at runtime. If the resolved command is empty (i.e. `$EDITOR` is not set and no explicit command is configured), the system SHALL return an error.

#### Scenario: Default $EDITOR
- **WHEN** config has no `note_open_command` field and `$EDITOR` is set to `nvim`
- **THEN** session notes are opened with `nvim`

#### Scenario: Custom command
- **WHEN** config has `note_open_command = "code"`
- **THEN** session notes are opened with `code` regardless of `$EDITOR`

#### Scenario: $EDITOR not set
- **WHEN** config has `note_open_command = "$EDITOR"` (or absent) and `$EDITOR` is not set
- **THEN** system returns an error: "$EDITOR is not set. Set it or configure note_open_command in config."
