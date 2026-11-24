//! fzf-side effect executor: turns domain [`Effect`]s into the files the
//! shell wrapper reads (`cd_file`, `post_cmd_file`, the `#EZ_RELAUNCH` marker)
//! and stderr messages.

use std::fs;
use std::path::Path;

use colored::Colorize;

use crate::config::model::EzConfig;
use crate::error::Result;
use crate::plugin;
use crate::repo::model::RepoEntry;
use crate::session;
use crate::session::model::Session;

use super::domain::{self, Effect};

/// Write the target directory for the shell wrapper to cd into.
pub(crate) fn write_cd_target(cd_file: Option<&Path>, target_dir: &Path) -> Result<()> {
    if let Some(cd_path) = cd_file {
        fs::write(cd_path, target_dir.to_string_lossy().as_bytes())?;
    } else {
        println!("{}", target_dir.display());
    }
    Ok(())
}

/// Write post-exit shell commands for the shell wrapper to source after ez exits.
pub(crate) fn write_post_commands(post_cmd_file: Option<&Path>, commands: &[String]) -> Result<()> {
    if commands.is_empty() {
        return Ok(());
    }
    if let Some(path) = post_cmd_file {
        fs::write(path, commands.join("\n"))?;
    }
    Ok(())
}

/// Append the `#EZ_RELAUNCH` marker to post_cmd_file, signaling the shell
/// wrapper to re-invoke `ez` after sourcing.
pub(crate) fn write_relaunch_marker(post_cmd_file: Option<&Path>) -> Result<()> {
    if let Some(path) = post_cmd_file {
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        writeln!(f, "\n#EZ_RELAUNCH")?;
    }
    Ok(())
}

/// Where the fzf front end sends effects.
pub(crate) struct EffectTarget<'a> {
    pub cd_file: Option<&'a Path>,
    pub post_cmd_file: Option<&'a Path>,
    pub config: &'a EzConfig,
}

/// Apply `effects` in order. Returns `true` when they asked to leave the browser.
///
/// `Refresh` needs no work here: the fzf loops rebuild their rows on every pass.
pub(crate) fn apply_effects(effects: &[Effect], target: &EffectTarget<'_>) -> Result<bool> {
    let mut exit = false;
    for effect in effects {
        log::debug!("apply_effect: {effect:?}");
        match effect {
            Effect::Cd(dir) => write_cd_target(target.cd_file, dir)?,
            Effect::PostCmds { cmds, relaunch } => {
                write_post_commands(target.post_cmd_file, cmds)?;
                if *relaunch {
                    write_relaunch_marker(target.post_cmd_file)?;
                }
            }
            Effect::Exec(cmds) => plugin::runner::run_shell_commands(cmds)?,
            Effect::OpenNote {
                repo_id,
                session_id,
            } => {
                if let Err(e) = session::notes::open_note(repo_id, session_id, target.config) {
                    eprintln!("{} {}", "Note open failed:".red(), e);
                }
            }
            Effect::Refresh(_) => {}
            Effect::Notify(text, _level) => eprintln!("{text}"),
            Effect::Exit => exit = true,
        }
    }
    Ok(exit)
}

/// Accept a session and apply the resulting effects (cd, plugin bind, env exports).
pub(crate) fn accept_session(
    on_enter: &str,
    repo_entry: &RepoEntry,
    selected: &Session,
    target_dir: &Path,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    config: &EzConfig,
) -> Result<()> {
    let effects = domain::accept::accept_session(
        on_enter,
        repo_entry,
        selected,
        target_dir,
        config,
        post_cmd_file.is_some(),
    );
    apply_effects(
        &effects,
        &EffectTarget {
            cd_file,
            post_cmd_file,
            config,
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::domain::{Level, RefreshScope};
    use std::path::PathBuf;

    fn run(effects: &[Effect], cd: Option<&Path>, post: Option<&Path>) -> bool {
        let config = EzConfig::default();
        apply_effects(
            effects,
            &EffectTarget {
                cd_file: cd,
                post_cmd_file: post,
                config: &config,
            },
        )
        .unwrap()
    }

    #[test]
    fn cd_post_cmds_and_exit_are_written() {
        let dir = tempfile::tempdir().unwrap();
        let cd = dir.path().join("cd");
        let post = dir.path().join("post");
        let exit = run(
            &[
                Effect::Cd(PathBuf::from("/some/where")),
                Effect::PostCmds {
                    cmds: vec!["echo a".into(), "echo b".into()],
                    relaunch: false,
                },
                Effect::Notify("hi".into(), Level::Info),
                Effect::Refresh(RefreshScope::Sessions),
                Effect::Exit,
            ],
            Some(&cd),
            Some(&post),
        );
        assert!(exit);
        assert_eq!(fs::read_to_string(&cd).unwrap(), "/some/where");
        assert_eq!(fs::read_to_string(&post).unwrap(), "echo a\necho b");
    }

    #[test]
    fn relaunch_marker_is_appended_after_commands() {
        let dir = tempfile::tempdir().unwrap();
        let post = dir.path().join("post");
        let exit = run(
            &[
                Effect::PostCmds {
                    cmds: vec!["echo a".into()],
                    relaunch: false,
                },
                Effect::PostCmds {
                    cmds: vec![],
                    relaunch: true,
                },
            ],
            None,
            Some(&post),
        );
        assert!(!exit);
        assert_eq!(fs::read_to_string(&post).unwrap(), "echo a\n#EZ_RELAUNCH\n");
    }

    #[test]
    fn relaunch_without_commands_creates_marker_only() {
        let dir = tempfile::tempdir().unwrap();
        let post = dir.path().join("post");
        run(
            &[Effect::PostCmds {
                cmds: vec![],
                relaunch: true,
            }],
            None,
            Some(&post),
        );
        assert_eq!(fs::read_to_string(&post).unwrap(), "\n#EZ_RELAUNCH\n");
    }

    #[test]
    fn post_cmds_without_file_are_ignored() {
        assert!(!run(
            &[Effect::PostCmds {
                cmds: vec!["echo a".into()],
                relaunch: true,
            }],
            None,
            None,
        ));
    }
}
