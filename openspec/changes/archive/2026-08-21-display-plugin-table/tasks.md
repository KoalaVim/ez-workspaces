## 1. Implementation

- [x] 1.1 Refactor `list_plugins()` in `src/plugin/mod.rs` to collect plugins into a `Vec<(String, String, String)>` (name, status, description) instead of printing inline
- [x] 1.2 Compute max column widths for name and status columns from the collected data
- [x] 1.3 Print header row ("Name", "Status", "Description") using computed widths
- [x] 1.4 Print separator line using `─` characters matching column widths
- [x] 1.5 Print each plugin row using computed widths, preserving cyan/green/dimmed coloring

## 2. Verification

- [x] 2.1 Build the project and run `ez plugin list` to confirm table output renders correctly
