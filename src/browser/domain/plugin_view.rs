//! Effects of choosing an item in a plugin view (`OnViewSelect` response).
//!
//! Mirrors the fzf plugin view: the cd target and post-exit commands go to
//! the shell wrapper, immediate commands run in the ez process, and the
//! browser exits afterwards.

use crate::plugin::protocol::HookResponse;

use super::{Effect, Level};

/// Turn an `OnViewSelect` response into effects. `has_post_cmd_file` tells
/// whether the shell wrapper passed `--post-cmd-file`; without it the
/// post-exit commands run inline, with a warning.
pub(crate) fn view_select_effects(resp: &HookResponse, has_post_cmd_file: bool) -> Vec<Effect> {
    let mut effects = Vec::new();
    if let Some(cd) = &resp.cd_target {
        effects.push(Effect::Cd(cd.clone()));
    }
    if !resp.post_shell_commands.is_empty() {
        if has_post_cmd_file {
            effects.push(Effect::PostCmds {
                cmds: resp.post_shell_commands.clone(),
                relaunch: false,
            });
        } else {
            effects.push(Effect::Notify(
                "warning: shell wrapper is outdated. Re-run: eval \"$(ez init-shell zsh)\"".into(),
                Level::Warn,
            ));
            effects.push(Effect::Exec(resp.post_shell_commands.clone()));
        }
    }
    if !resp.shell_commands.is_empty() {
        effects.push(Effect::Exec(resp.shell_commands.clone()));
    }
    effects.push(Effect::Exit);
    effects
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn response() -> HookResponse {
        serde_json::from_str(
            r#"{"success": true, "cd_target": "/x", "post_shell_commands": ["a"], "shell_commands": ["b"]}"#,
        )
        .unwrap()
    }

    #[test]
    fn effects_follow_the_fzf_order_and_exit() {
        assert_eq!(
            view_select_effects(&response(), true),
            vec![
                Effect::Cd(PathBuf::from("/x")),
                Effect::PostCmds {
                    cmds: vec!["a".into()],
                    relaunch: false
                },
                Effect::Exec(vec!["b".into()]),
                Effect::Exit,
            ]
        );
    }

    #[test]
    fn outdated_wrapper_runs_post_commands_inline() {
        let effects = view_select_effects(&response(), false);
        assert!(matches!(effects[1], Effect::Notify(..)));
        assert_eq!(effects[2], Effect::Exec(vec!["a".into()]));
    }
}
