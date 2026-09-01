## Purpose

Keybind in the session picker to switch to the global browser view.

## Requirements

### Requirement: Session picker has a keybind to switch to the global browser
The session picker SHALL provide a configurable keybind (default: `ctrl-a`) that exits the session picker and opens the global browser view, equivalent to running `ez --all`.

#### Scenario: User presses the view-all keybind in the session picker
- **WHEN** the user is in the session picker (entered via auto-detect or `--repo`) and presses the `view_all` keybind
- **THEN** the session picker closes and the global browser view opens with the default view mode

#### Scenario: User cancels after switching to the global browser
- **WHEN** the user presses the `view_all` keybind and then presses Escape in the global browser
- **THEN** ez exits normally (does not return to the original session picker)

### Requirement: The view-all keybind is configurable
The keybind SHALL be configurable via the `keybinds.view_all` field in the config file, defaulting to `ctrl-a`.

#### Scenario: Custom keybind configured
- **WHEN** the user sets `keybinds.view_all = "ctrl-b"` in their config
- **THEN** pressing `ctrl-b` in the session picker switches to the global browser view
