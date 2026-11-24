//! Preview output split into fast text and slow sections.
//!
//! Renderers write into a [`Sink`]. In full mode (`ez preview`, fzf) slow
//! sections (git status, git log, bat-rendered notes, directory branch
//! lookups, plugin preview commands) are computed inline, so the output is
//! the same as before the split. In deferred mode (native TUI) they are
//! recorded as [`Segment::Slow`] so the front end can show the fast text
//! immediately and splice each section in when a worker finished it.

use std::fmt::{self, Write};
use std::fs;
use std::path::{Path, PathBuf};

use colored::Colorize;

use super::super::git::{get_branch, git_cmd};
use super::keybind_help::preview_section;

/// A section whose rendering needs slow work (git, external programs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Slow {
    /// Branch and dirty-file count of a worktree.
    GitInfo(PathBuf),
    /// Last commits; `none_text` prints "no commits" when the log is empty.
    Commits { path: PathBuf, none_text: bool },
    /// A session note rendered with `bat`.
    Note(PathBuf),
    /// Directory listing with a branch lookup per repo.
    DirContents(PathBuf),
    /// A plugin view preview command (`sh -c`).
    Shell(String),
}

/// A piece of preview output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    Text(String),
    Slow(Slow),
}

/// Output buffer of the preview renderers.
pub struct Sink {
    text: String,
    segments: Vec<Segment>,
    deferred: bool,
}

impl Sink {
    /// Slow sections are rendered inline.
    pub fn full() -> Self {
        Self {
            text: String::new(),
            segments: Vec::new(),
            deferred: false,
        }
    }

    /// Slow sections are recorded as segments.
    pub fn deferred() -> Self {
        Self {
            deferred: true,
            ..Self::full()
        }
    }

    /// Emit a slow section.
    pub fn slow(&mut self, slow: Slow) {
        if self.deferred {
            self.flush();
            self.segments.push(Segment::Slow(slow));
        } else {
            self.text.push_str(&slow.render());
        }
    }

    fn flush(&mut self) {
        if !self.text.is_empty() {
            self.segments
                .push(Segment::Text(std::mem::take(&mut self.text)));
        }
    }

    /// The text of a full-mode sink.
    pub fn into_text(self) -> String {
        self.text
    }

    /// The segments of a deferred sink.
    pub fn into_segments(mut self) -> Vec<Segment> {
        self.flush();
        self.segments
    }
}

impl Write for Sink {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.text.push_str(s);
        Ok(())
    }
}

impl Slow {
    /// Render the section body (may block on git or external programs).
    pub fn render(&self) -> String {
        let mut out = String::new();
        match self {
            Slow::GitInfo(path) => git_info(&mut out, path),
            Slow::Commits { path, none_text } => commits(&mut out, path, *none_text),
            Slow::Note(path) => note(&mut out, path),
            Slow::DirContents(path) => dir_contents(&mut out, path),
            Slow::Shell(cmd) => return run_shell(cmd),
        }
        out
    }
}

fn git_info(out: &mut String, path: &Path) {
    let branch = get_branch(path).unwrap_or_else(|| "detached".into());
    outln!(out, "  {} {}", "branch:".bold(), branch.cyan());

    let dirty_count = git_cmd(path, &["status", "--porcelain"])
        .map(|s| s.lines().count())
        .unwrap_or(0);
    if dirty_count > 0 {
        outln!(
            out,
            "  {} {}",
            "status:".bold(),
            format!("{dirty_count} modified file(s)").yellow()
        );
    } else {
        outln!(out, "  {} {}", "status:".bold(), "clean".green());
    }
}

fn commits(out: &mut String, path: &Path, none_text: bool) {
    if let Some(log) = git_cmd(
        path,
        &["log", "--oneline", "--decorate", "--no-color", "-8"],
    ) {
        for line in log.lines() {
            if let Some((hash, msg)) = line.split_once(' ') {
                outln!(out, "  {} {}", hash.yellow(), msg);
            } else {
                outln!(out, "  {line}");
            }
        }
    } else if none_text {
        outln!(out, "  {}", "no commits".dimmed());
    }
}

fn note(out: &mut String, readme: &Path) {
    let Some(output) = render_note_with_bat(readme) else {
        return;
    };
    if output.trim().is_empty() {
        return;
    }
    outln!(out);
    preview_section(out, "Note");
    for line in output.lines() {
        outln!(out, "  {line}");
    }
}

/// Render a README.md via `bat` with syntax highlighting.
/// Returns None if `bat` is not installed or the command fails.
fn render_note_with_bat(path: &Path) -> Option<String> {
    std::process::Command::new("bat")
        .args([
            "--style=plain",
            "--color=always",
            "--line-range=:20",
            &path.display().to_string(),
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
}

fn dir_contents(out: &mut String, path: &Path) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    let mut repos: Vec<String> = Vec::new();
    let mut dirs: Vec<String> = Vec::new();
    let mut file_count: usize = 0;

    for entry in entries.flatten() {
        let p = entry.path();
        let entry_name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if entry_name.starts_with('.') {
            continue;
        }

        if p.is_dir() {
            if p.join(".git").exists() {
                let branch = get_branch(&p).unwrap_or_else(|| "?".into());
                repos.push(format!(
                    "  {} {} {}",
                    "●".green(),
                    entry_name.bold(),
                    format!("[{branch}]").cyan()
                ));
            } else {
                dirs.push(format!("  {} {}/", "▸".blue(), entry_name.bold().blue()));
            }
        } else {
            file_count += 1;
        }
    }

    repos.sort();
    dirs.sort();

    if !repos.is_empty() {
        outln!(
            out,
            "  {}",
            format!("Repositories ({})", repos.len()).bold().green()
        );
        for r in &repos {
            outln!(out, "{r}");
        }
    }

    if !dirs.is_empty() {
        if !repos.is_empty() {
            outln!(out);
        }
        outln!(
            out,
            "  {}",
            format!("Directories ({})", dirs.len()).bold().blue()
        );
        for d in &dirs {
            outln!(out, "{d}");
        }
    }

    if file_count > 0 {
        outln!(out);
        outln!(out, "  {} {file_count} file(s)", "…".dimmed());
    }

    if repos.is_empty() && dirs.is_empty() && file_count == 0 {
        outln!(out, "  {}", "(empty)".dimmed());
    }
}

/// Run a plugin view's preview command like fzf does (`sh -c`, output shown as is).
fn run_shell(cmd: &str) -> String {
    log::debug!("preview: shell {cmd:?}");
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdin(std::process::Stdio::null())
        .output()
    {
        Ok(o) => {
            let mut text = String::from_utf8_lossy(&o.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&o.stderr));
            text
        }
        Err(e) => format!("preview command failed: {e}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_mode_inlines_slow_sections_and_deferred_records_them() {
        let dir = tempfile::tempdir().unwrap();
        let slow = Slow::DirContents(dir.path().to_path_buf());
        let mut full = Sink::full();
        outln!(full, "head");
        full.slow(slow.clone());
        outln!(full, "tail");
        let text = full.into_text();
        assert_eq!(text, format!("head\n{}tail\n", slow.render()));

        let mut deferred = Sink::deferred();
        outln!(deferred, "head");
        deferred.slow(slow.clone());
        outln!(deferred, "tail");
        assert_eq!(
            deferred.into_segments(),
            vec![
                Segment::Text("head\n".into()),
                Segment::Slow(slow),
                Segment::Text("tail\n".into()),
            ]
        );
    }

    #[test]
    fn shell_section_returns_command_output() {
        assert_eq!(Slow::Shell("echo hi".into()).render(), "hi\n");
    }
}
