## MODIFIED Requirements

### Requirement: Session tree hierarchy
Sessions SHALL be organized in a tree using `parent_id` pointers. The system SHALL support operations: list roots, find children, find ancestors, find descendants, and render as an indented tree.

#### Scenario: Render tree
- **WHEN** user runs `ez session list`
- **THEN** system renders sessions as an indented tree with root sessions at the top level and children indented below their parents
- **THEN** session names SHALL NOT include a path suffix (no `→ <path>` display)

#### Scenario: Flat list
- **WHEN** user runs `ez session list --flat`
- **THEN** system renders sessions as a flat list without tree structure
- **THEN** session names SHALL NOT include a path suffix (no `(<path>)` display)

#### Scenario: JSON session list
- **WHEN** user runs `ez session list --json --repo my-repo`
- **THEN** system outputs a JSON array of session objects with fields: `id`, `name`, `parent_id`, `path`, `bare`, `labels`, `last_accessed`, `env`, `is_default`

#### Scenario: Session path in interactive browser
- **WHEN** sessions are listed in the interactive fzf browser
- **THEN** session items SHALL NOT include a `→ <path>` suffix after the session name
