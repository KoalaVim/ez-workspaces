# Clone from Browser

## Purpose

Allow users to clone a git repository directly from the workspace drill-down directory browser, without leaving the interactive browser. After cloning, the repo is auto-registered and the user transitions into its session picker.

## Requirements

### Requirement: Clone repo from workspace drill-down
The browser SHALL support a keybind in the workspace drill-down directory view that prompts for a git URL and clones the repository into the currently browsed directory. After a successful clone, the system SHALL auto-register the repo and transition directly into its session picker. If the clone fails, the system SHALL display the error and return to the directory listing.

#### Scenario: Successful clone from directory browser
- **WHEN** user presses the clone keybind (default `alt-a`) while browsing a workspace directory
- **THEN** system prompts for a git URL via the interactive input
- **AND** clones the repo into a subdirectory of the current directory (name derived from URL)
- **AND** registers the repo in the global index
- **AND** transitions into the cloned repo's session picker

#### Scenario: Clone failure returns to browser
- **WHEN** user enters an invalid URL or git clone fails
- **THEN** system displays the error message
- **AND** returns to the directory listing so the user can retry or continue browsing

#### Scenario: Empty URL cancels clone
- **WHEN** user presses Escape or enters an empty string at the URL prompt
- **THEN** system cancels the clone and returns to the directory listing

#### Scenario: Clone keybind shown in preview
- **WHEN** user is browsing directories in the workspace view and views the preview pane
- **THEN** the keybind help includes the clone repo keybind with its description

### Requirement: Configurable clone keybind
The clone keybind SHALL be configurable via the `[keybinds]` section in the config file under the key `clone_repo`. The default value SHALL be `alt-a`.

#### Scenario: Custom keybind
- **WHEN** user sets `clone_repo = "alt-g"` in their config
- **THEN** the clone action is triggered by `alt-g` instead of `alt-a` in the workspace drill-down
