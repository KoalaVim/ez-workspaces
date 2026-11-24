//! Native effect executor. Mirrors `browser::effects` (the fzf executor) and
//! reuses its file-writing helpers so `cd_file`, `post_cmd_file` and the
//! `#EZ_RELAUNCH` marker are byte-identical. Differences forced by the alternate
//! screen: messages become notices instead of stderr output, anything that
//! needs the terminal runs with the TUI suspended, and stdout lines are held
//! back until the TUI has been torn down.

use std::path::Path;

use super::terminal::SharedGuard;
use crate::browser::domain::{Effect, Level};
use crate::browser::effects::{write_cd_target, write_post_commands, write_relaunch_marker};
use crate::config::model::EzConfig;
use crate::error::Result;
use crate::plugin;
use crate::session;

/// Gives programs the real terminal for a while.
pub trait Release {
    fn run_released(&self, f: &mut dyn FnMut()) -> Result<()>;
}

impl Release for SharedGuard {
    /// Leave the alternate screen, run `f` with inherited stdio, come back and repaint.
    fn run_released(&self, f: &mut dyn FnMut()) -> Result<()> {
        self.borrow_mut().suspend()?;
        f();
        self.borrow_mut().resume()
    }
}

/// Where the native front end sends effects.
pub struct Target<'a> {
    pub cd_file: Option<&'a Path>,
    pub post_cmd_file: Option<&'a Path>,
    pub config: &'a EzConfig,
}

/// What the app has to do after the effects ran.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Applied {
    /// Leave the browser.
    pub exit: bool,
    /// Rows changed: reload.
    pub refresh: bool,
    /// Messages for the notice line, in order.
    pub notices: Vec<(String, Level)>,
    /// Lines to print on stdout once the TUI is gone (cd target without `cd_file`).
    pub stdout_after: Vec<String>,
}

/// Apply `effects` in order.
pub fn apply(effects: &[Effect], term: &dyn Release, target: &Target<'_>) -> Result<Applied> {
    let mut out = Applied::default();
    for effect in effects {
        log::debug!("tui apply_effect: {effect:?}");
        match effect {
            Effect::Cd(dir) => match target.cd_file {
                Some(_) => write_cd_target(target.cd_file, dir)?,
                None => out.stdout_after.push(dir.display().to_string()),
            },
            Effect::PostCmds { cmds, relaunch } => {
                write_post_commands(target.post_cmd_file, cmds)?;
                if *relaunch {
                    write_relaunch_marker(target.post_cmd_file)?;
                }
            }
            Effect::Exec(cmds) => {
                let mut result = Ok(());
                term.run_released(&mut || result = plugin::runner::run_shell_commands(cmds))?;
                if let Err(e) = result {
                    out.notices
                        .push((format!("Command failed: {e}"), Level::Error));
                }
            }
            Effect::OpenNote {
                repo_id,
                session_id,
            } => {
                let mut result = Ok(());
                term.run_released(&mut || {
                    result = session::notes::open_note(repo_id, session_id, target.config)
                })?;
                if let Err(e) = result {
                    out.notices
                        .push((format!("Note open failed: {e}"), Level::Error));
                }
            }
            Effect::Refresh(_) => out.refresh = true,
            Effect::Notify(text, level) => out.notices.push((text.clone(), *level)),
            Effect::Exit => out.exit = true,
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::domain::RefreshScope;
    use std::cell::Cell;
    use std::fs;
    use std::path::PathBuf;

    #[derive(Default)]
    struct FakeTerm {
        released: Cell<u32>,
    }

    impl Release for FakeTerm {
        fn run_released(&self, f: &mut dyn FnMut()) -> Result<()> {
            self.released.set(self.released.get() + 1);
            f();
            Ok(())
        }
    }

    fn run(effects: &[Effect], cd: Option<&Path>, post: Option<&Path>, term: &FakeTerm) -> Applied {
        let config = EzConfig::default();
        apply(
            effects,
            term,
            &Target {
                cd_file: cd,
                post_cmd_file: post,
                config: &config,
            },
        )
        .unwrap()
    }

    #[test]
    fn cd_post_cmds_and_exit_match_the_fzf_files() {
        let dir = tempfile::tempdir().unwrap();
        let (cd, post) = (dir.path().join("cd"), dir.path().join("post"));
        let term = FakeTerm::default();
        let applied = run(
            &[
                Effect::Cd(PathBuf::from("/some/where")),
                Effect::PostCmds {
                    cmds: vec!["echo a".into(), "echo b".into()],
                    relaunch: true,
                },
                Effect::Notify("hi".into(), Level::Info),
                Effect::Refresh(RefreshScope::Sessions),
                Effect::Exit,
            ],
            Some(&cd),
            Some(&post),
            &term,
        );
        assert!(applied.exit && applied.refresh);
        assert_eq!(applied.notices, vec![("hi".to_string(), Level::Info)]);
        assert_eq!(fs::read_to_string(&cd).unwrap(), "/some/where");
        assert_eq!(
            fs::read_to_string(&post).unwrap(),
            "echo a\necho b\n#EZ_RELAUNCH\n"
        );
        assert_eq!(term.released.get(), 0);
    }

    #[test]
    fn cd_without_file_is_printed_after_teardown() {
        let applied = run(
            &[Effect::Cd(PathBuf::from("/x")), Effect::Exit],
            None,
            None,
            &FakeTerm::default(),
        );
        assert_eq!(applied.stdout_after, vec!["/x".to_string()]);
    }

    #[test]
    fn exec_runs_with_the_terminal_released() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("marker");
        let term = FakeTerm::default();
        let cmd = format!("echo ran > '{}'", marker.display());
        let applied = run(&[Effect::Exec(vec![cmd])], None, None, &term);
        assert_eq!(term.released.get(), 1);
        assert!(applied.notices.is_empty());
        assert_eq!(fs::read_to_string(&marker).unwrap().trim(), "ran");
    }
}
