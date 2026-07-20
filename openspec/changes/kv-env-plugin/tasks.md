## 1. Plugin Script

- [x] 1.1 Create `plugins/kv/manifest.toml` with hooks (on_session_create, on_session_delete, on_session_enter, on_session_rename), config_schema for `source_env` (string, default "main")
- [x] 1.2 Create `plugins/kv/kv-plugin` bash script implementing all four hooks: fork-on-create, delete-on-delete, env-inject-on-enter, rename-on-rename. Include `kv` availability check, default-session skip, debug logging (matching existing plugin patterns), and `jq`-based JSON responses.

## 2. Bundle Registration

- [x] 2.1 Add kv plugin entry to `BUNDLED_PLUGINS` array in `src/plugin/bundled.rs` with `include_str!` for manifest and executable

## 3. Documentation

- [x] 3.1 Update `README.md` to mention the kv plugin in the plugins section
- [x] 3.2 Update `docs/user-guide.md` to document the kv plugin (enable command, config options, what it does)
- [x] 3.3 Update `AGENTS.md` to reference the kv plugin in the bundled plugins list

## 4. Verification

- [x] 4.1 Run `make check` to verify build compiles with zero warnings, tests pass, and formatting is correct
