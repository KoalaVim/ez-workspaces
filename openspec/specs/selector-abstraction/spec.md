# Selector Abstraction

## Purpose

Decouple the interactive UI from any specific backend (fzf, skim, ratatui, etc.) via the `InteractiveSelector` trait. All browser and session management code operates through this trait, enabling backend substitution without changing business logic. The current implementation is `FzfSelector`.

## Requirements

### Requirement: InteractiveSelector trait
The system SHALL define an `InteractiveSelector` trait with 7 methods that cover all interactive UI patterns: single selection, multi selection, free-text input, confirmation, selection with back navigation, input with back navigation, and selection with action keybinds.

#### Scenario: Trait is the only UI boundary
- **WHEN** browser or session code needs user interaction
- **THEN** it calls methods on `&dyn InteractiveSelector`, never references a concrete backend type

### Requirement: Single selection
The trait SHALL provide `select_one` to present a list of items and return the selected index. It accepts an optional preview command string. It SHALL return `None` on cancel (Escape/Ctrl-C).

#### Scenario: Select from list
- **WHEN** `select_one` is called with items and a prompt
- **THEN** the backend presents the items, user picks one, and the method returns `Some(index)`

#### Scenario: Cancel returns None
- **WHEN** user presses Escape during `select_one`
- **THEN** method returns `Ok(None)`

### Requirement: Multi selection
The trait SHALL provide `select_many` to present items with multi-select capability (Tab to toggle in fzf). It SHALL return a vector of selected indices.

#### Scenario: Multi-select
- **WHEN** `select_many` is called
- **THEN** user can toggle multiple items and confirm; method returns all selected indices

### Requirement: Free-text input
The trait SHALL provide `input` for free-text entry with an optional default value. The implementation uses fzf with `--print-query` and no items.

#### Scenario: Text input with default
- **WHEN** `input` is called with `default: Some("feature-x")`
- **THEN** the prompt pre-fills with `feature-x`; user can edit or accept

### Requirement: Confirmation
The trait SHALL provide `confirm` for yes/no prompts with a default. The implementation reads from stdin directly (not through fzf).

#### Scenario: Confirm with default yes
- **WHEN** `confirm` is called with `default: true`
- **THEN** prompt shows `[Y/n]`; empty input returns `true`

### Requirement: Selection with back navigation
The trait SHALL provide `select_with_back` for multi-stage flows. It returns a `StageOutcome`: `Picked(value)`, `Back`, or `Cancel`. When `allow_back` is true, Ctrl-P triggers `Back`. An optional `context` string shows accumulated progress above the prompt.

#### Scenario: Pick from list
- **WHEN** user selects an item
- **THEN** method returns `StageOutcome::Picked(value)`

#### Scenario: Back navigation
- **WHEN** user presses Ctrl-P
- **THEN** method returns `StageOutcome::Back`

#### Scenario: Typed query accepted
- **WHEN** user types a custom value that doesn't match any item and presses Enter
- **THEN** method returns `StageOutcome::Picked(typed_query)`

### Requirement: Input with back navigation
The trait SHALL provide `input_with_back` for free-text stages that support Ctrl-P back navigation. Returns `StageOutcome` like `select_with_back`.

#### Scenario: Free text with back
- **WHEN** user types text and presses Enter
- **THEN** method returns `StageOutcome::Picked(text)`

#### Scenario: Back from text input
- **WHEN** user presses Ctrl-P during text input
- **THEN** method returns `StageOutcome::Back`

### Requirement: Selection with action keybinds
The trait SHALL provide `select_with_actions` for the session action loop and view dispatch. It accepts a list of expected keybind strings and returns an `ActionResult`: `Select(index)` for Enter, `Action(key, index)` for a keybind press, or `Cancel` for Escape.

#### Scenario: Enter selects
- **WHEN** user presses Enter on an item
- **THEN** method returns `ActionResult::Select(index)`

#### Scenario: Keybind action
- **WHEN** user presses Alt-n on an item
- **THEN** method returns `ActionResult::Action("alt-n", index)`

#### Scenario: Cancel
- **WHEN** user presses Escape
- **THEN** method returns `ActionResult::Cancel`

### Requirement: Data types are backend-agnostic
`SelectItem` (display + value strings), `ActionResult`, and `StageOutcome` SHALL contain no backend-specific data. They use plain strings and indices.

#### Scenario: SelectItem structure
- **WHEN** items are constructed for any selector method
- **THEN** each item has a `display` string (what the user sees, may contain ANSI colors) and a `value` string (internal identifier)

### Requirement: FzfSelector implementation
The `FzfSelector` SHALL implement `InteractiveSelector` by spawning fzf processes. It SHALL use tab-delimited value prefixing with `--with-nth` to separate display from value, `--preview` for preview commands, `--expect` for action keybinds, and `--print-query` for typed input.

#### Scenario: Value prefix matching
- **WHEN** preview is enabled or actions are used
- **THEN** fzf receives `value\tdisplay` lines and matches on the value field (first tab-delimited column) to avoid ANSI stripping issues

#### Scenario: Fzf availability check
- **WHEN** `FzfSelector::new` is called
- **THEN** system verifies fzf is in PATH; if not, returns `SelectorUnavailable` error

### Requirement: Single injection point
The concrete selector SHALL be instantiated in exactly one place (`browse()` function) based on the config. All other code receives `&dyn InteractiveSelector`.

#### Scenario: Backend selection
- **WHEN** the browser starts
- **THEN** `browse()` creates a `FzfSelector` from `config.fzf` settings and passes it as `&dyn InteractiveSelector` to all downstream functions

### Requirement: Fzf-specific configuration
The `FzfSelector` SHALL accept configuration for height (`fzf.height`, default `90%`) and extra opts (`fzf.extra_opts`). Extra opts are appended as whitespace-split arguments to every fzf invocation.

#### Scenario: Custom height
- **WHEN** config has `[fzf] height = "100%"`
- **THEN** all fzf invocations use `--height 100%`

#### Scenario: Extra opts
- **WHEN** config has `[fzf] extra_opts = "--border --info=inline"`
- **THEN** those flags are appended to every fzf command
