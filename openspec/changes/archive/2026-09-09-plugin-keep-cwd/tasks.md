## 1. Protocol

- [x] 1.1 Add `keep_cwd: bool` to `HookResponse` in `src/plugin/protocol.rs` with `#[serde(default, skip_serializing_if = "std::ops::Not::not")]` and a doc comment ("plugin navigated itself; ez must not cd the invoking shell")
- [x] 1.2 Extend the existing `HookResponse` parse test to assert `keep_cwd` defaults to false when absent and parses true when present

## 2. accept_session

- [x] 2.1 In `accept_session` (`src/browser/mod.rs`), when `response.keep_cwd` is true: skip the implicit `write_cd_target(cd_file, target_dir)` at the `cd_target.is_none()` branch, and skip the trailing fallback `write_cd_target`; still write `post_shell_commands` + session env exports
- [x] 2.2 Update the doc comment on `accept_session` to describe `keep_cwd`
- [x] 2.3 Add a unit test covering the cd decision for a response with `post_shell_commands` and `keep_cwd: true` (no cd-file write) vs the same response without it (cd-file written)

## 3. Herdr plugin

- [x] 3.1 In `plugins/herdr/herdr-plugin`, `on_bind` `INSIDE_HERDR` branch: add `keep_cwd: true` to the jq response, with a short comment on why (focus moves inside herdr, the invoking shell stays put)
- [x] 3.2 Verify the non-herdr `on_bind` branch and all other hooks are unchanged

## 4. Docs & verification

- [x] 4.1 Document `keep_cwd` in `docs/plugin-guide.md` next to `cd_target` (response fields list and the hook-flow section)
- [x] 4.2 `make check` (fmt, clippy, tests) passes
- [x] 4.3 Manual (verified locally by the user): with `on_create = "herdr"` inside herdr, `ez session new` opens/focuses the workspace and the creating shell's cwd is unchanged; with `on_create = "tmux"` the cd still happens
