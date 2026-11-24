//! Accepting a session (cd into it or run a plugin bind) as effects.

use std::collections::HashMap;
use std::path::Path;

use colored::Colorize;

use crate::config::model::EzConfig;
use crate::plugin;
use crate::plugin::protocol::HookResponse;
use crate::repo::model::RepoEntry;
use crate::session::model::Session;

use super::{Effect, Level};

/// Generate `export` commands for a session's env map.
pub(crate) fn session_env_exports(env: &HashMap<String, String>) -> Vec<String> {
    let mut exports: Vec<String> = env
        .iter()
        .map(|(k, v)| format!("export {}={}", k, shell_escape(v)))
        .collect();
    exports.sort();
    exports
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Returns true if the `on_create` value means "do nothing" (skip navigation after create).
pub(crate) fn on_create_is_noop(v: &str) -> bool {
    v.is_empty() || v == "none"
}

/// Effects of a plugin HookResponse (cd target, post-exit commands, inline commands).
///
/// `post_cmds_supported` tells whether the front end can hand post-exit
/// commands to the shell wrapper; when it cannot they run inline instead.
pub(crate) fn bind_response_effects(
    response: &HookResponse,
    post_cmds_supported: bool,
) -> Vec<Effect> {
    let mut effects = Vec::new();
    // A plugin that reports failure used to be silently ignored, leaving the
    // keybind looking dead. Surface it instead.
    if !response.success {
        let reason = response.error.as_deref().unwrap_or("no reason given");
        effects.push(Effect::Notify(
            format!("{} {}", "plugin bind failed:".red(), reason),
            Level::Error,
        ));
        log::debug!("bind response reported failure: {reason}");
    }
    if let Some(ref cd) = response.cd_target {
        effects.push(Effect::Cd(cd.clone()));
    }
    if !response.post_shell_commands.is_empty() {
        if post_cmds_supported {
            effects.push(Effect::PostCmds {
                cmds: response.post_shell_commands.clone(),
                relaunch: false,
            });
        } else {
            effects.push(Effect::Notify(
                "warning: shell wrapper is outdated. Re-run: eval \"$(ez init-shell zsh)\""
                    .to_string(),
                Level::Warn,
            ));
            effects.push(Effect::Exec(response.post_shell_commands.clone()));
        }
    }
    if !response.shell_commands.is_empty() {
        effects.push(Effect::Exec(response.shell_commands.clone()));
    }
    effects
}

/// Whether a bind response navigated the user somewhere, so `accept_session` must not fall
/// back to a plain cd. `keep_cwd` counts: the plugin moved focus inside its own multiplexer.
pub(crate) fn bind_handled_navigation(response: &HookResponse) -> bool {
    response.cd_target.is_some() || !response.post_shell_commands.is_empty() || response.keep_cwd
}

fn cd_effects(target_dir: &Path, env_exports: Vec<String>) -> Vec<Effect> {
    let mut effects = vec![Effect::Cd(target_dir.to_path_buf())];
    if !env_exports.is_empty() {
        effects.push(Effect::PostCmds {
            cmds: env_exports,
            relaunch: false,
        });
    }
    effects.push(Effect::Exit);
    effects
}

/// Accept a session: either cd into it (default) or run a named plugin bind's hook.
///
/// The `on_enter` value is matched against each session-context bind's `label`, `bind_name`,
/// and `plugin_name` (in that order). On a match the bind's hook is invoked; if the response
/// carries a `cd_target` or `post_shell_commands` the effects of that response are returned.
/// When there is no match, or the bind produces no navigation effect, the result falls
/// back to a plain `cd` into `target_dir`.
///
/// A response with `keep_cwd` suppresses every cd this function would emit on its own — the
/// plugin navigated elsewhere (its own multiplexer workspace), so the invoking shell stays put.
/// An explicit `cd_target` still wins; env exports are emitted either way.
///
/// The returned effects always end with [`Effect::Exit`].
pub(crate) fn accept_session(
    on_enter: &str,
    repo_entry: &RepoEntry,
    selected: &Session,
    target_dir: &Path,
    config: &EzConfig,
    post_cmds_supported: bool,
) -> Vec<Effect> {
    // Propagate session env to the process so child processes (e.g. tmux new-session) inherit them
    for (k, v) in &selected.env {
        std::env::set_var(k, v);
    }
    let env_exports = session_env_exports(&selected.env);

    if on_enter == "cd" {
        return cd_effects(target_dir, env_exports);
    }

    // Look for a matching session-context plugin bind by label, bind_name, or plugin_name.
    let binds = plugin::collect_plugin_binds("session", config).unwrap_or_default();
    let matching = binds
        .iter()
        .find(|b| b.label == on_enter || b.bind_name == on_enter || b.plugin_name == on_enter);

    if let Some(bind) = matching {
        match plugin::run_bind_hook(
            &bind.plugin_name,
            &bind.bind_name,
            &bind.key,
            "session",
            &selected.id,
            &selected.name,
            repo_entry,
            Some(selected),
            config,
        ) {
            Ok(response) => {
                if bind_handled_navigation(&response) {
                    let mut effects = Vec::new();
                    if response.cd_target.is_none() && !response.keep_cwd {
                        effects.push(Effect::Cd(target_dir.to_path_buf()));
                    }
                    let mut all_commands = response.post_shell_commands.clone();
                    all_commands.extend(env_exports);
                    let merged = HookResponse {
                        post_shell_commands: all_commands,
                        ..response
                    };
                    effects.extend(bind_response_effects(&merged, post_cmds_supported));
                    effects.push(Effect::Exit);
                    return effects;
                }
            }
            Err(e) => {
                log::debug!("on_enter bind hook failed, falling back to cd: {e}");
            }
        }
    } else {
        log::debug!("on_enter=\"{on_enter}\": no matching session bind found, falling back to cd");
    }

    cd_effects(target_dir, env_exports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn response(post: &[&str], cd: Option<&str>, keep_cwd: bool) -> HookResponse {
        HookResponse {
            success: true,
            post_shell_commands: post.iter().map(|s| s.to_string()).collect(),
            cd_target: cd.map(PathBuf::from),
            keep_cwd,
            ..Default::default()
        }
    }

    /// The `on_create = herdr` case: herdr focused its own workspace, so ez writes no cd.
    #[test]
    fn keep_cwd_suppresses_ez_cd() {
        let r = response(&["herdr worktree open ..."], None, true);
        assert!(bind_handled_navigation(&r));
        assert!(r.cd_target.is_none() && r.keep_cwd);
    }

    /// Same response without keep_cwd (tmux, zellij): ez still cds into the worktree.
    #[test]
    fn without_keep_cwd_ez_cds() {
        let r = response(&["tmux attach -t foo"], None, false);
        assert!(bind_handled_navigation(&r));
        assert!(r.cd_target.is_none() && !r.keep_cwd);
    }

    /// An explicit cd_target wins over keep_cwd.
    #[test]
    fn explicit_cd_target_wins() {
        let r = response(&[], Some("/tmp/elsewhere"), true);
        assert!(bind_handled_navigation(&r));
        assert_eq!(r.cd_target, Some(PathBuf::from("/tmp/elsewhere")));
    }

    /// A no-op response leaves the fallback cd in place.
    #[test]
    fn no_effect_falls_back_to_cd() {
        assert!(!bind_handled_navigation(&response(&[], None, false)));
    }

    #[test]
    fn env_exports_are_sorted_and_escaped() {
        let mut env = HashMap::new();
        env.insert("B".to_string(), "it's".to_string());
        env.insert("A".to_string(), "x".to_string());
        assert_eq!(
            session_env_exports(&env),
            vec!["export A='x'", "export B='it'\\''s'"]
        );
    }

    #[test]
    fn bind_effects_cd_post_and_exec_in_order() {
        let mut r = response(&["tmux attach"], Some("/tmp/x"), false);
        r.shell_commands = vec!["echo hi".into()];
        assert_eq!(
            bind_response_effects(&r, true),
            vec![
                Effect::Cd(PathBuf::from("/tmp/x")),
                Effect::PostCmds {
                    cmds: vec!["tmux attach".into()],
                    relaunch: false
                },
                Effect::Exec(vec!["echo hi".into()]),
            ]
        );
    }

    #[test]
    fn bind_effects_run_post_cmds_inline_without_wrapper() {
        let effects = bind_response_effects(&response(&["a", "b"], None, false), false);
        assert!(matches!(effects[0], Effect::Notify(_, Level::Warn)));
        assert_eq!(effects[1], Effect::Exec(vec!["a".into(), "b".into()]));
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn failed_bind_notifies_error() {
        let r = HookResponse {
            success: false,
            error: Some("boom".into()),
            ..Default::default()
        };
        let effects = bind_response_effects(&r, true);
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            Effect::Notify(msg, Level::Error) => assert!(msg.contains("boom")),
            other => panic!("unexpected effect {other:?}"),
        }
    }

    #[test]
    fn cd_on_enter_emits_cd_exports_exit() {
        let mut env = HashMap::new();
        env.insert("K".to_string(), "v".to_string());
        let effects = cd_effects(Path::new("/w"), session_env_exports(&env));
        assert_eq!(
            effects,
            vec![
                Effect::Cd(PathBuf::from("/w")),
                Effect::PostCmds {
                    cmds: vec!["export K='v'".into()],
                    relaunch: false
                },
                Effect::Exit,
            ]
        );
        assert_eq!(
            cd_effects(Path::new("/w"), vec![]),
            vec![Effect::Cd(PathBuf::from("/w")), Effect::Exit]
        );
    }

    #[test]
    fn noop_on_create_values() {
        assert!(on_create_is_noop(""));
        assert!(on_create_is_noop("none"));
        assert!(!on_create_is_noop("cd"));
    }
}
