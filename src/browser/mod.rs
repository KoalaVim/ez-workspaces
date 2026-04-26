pub mod preview;
pub mod selector;
pub mod views;

use std::fs;
use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::plugin;
use crate::repo;
use crate::session;
use selector::{ActionResult, FzfSelector, InteractiveSelector, SelectItem};

pub use preview::preview;

/// Main interactive browser entry point (bare `ez` command).
pub fn browse(
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    workspace: Option<&str>,
    repo_flag: Option<&Path>,
    select_by: Option<&str>,
) -> Result<()> {
    let config = config::load()?;
    let selector = FzfSelector::new(&config.fzf)?;

    // --repo: jump straight to session picker for a specific repo
    if let Some(repo_path) = repo_flag {
        let repo_path = if repo_path.is_absolute() {
            repo_path.to_path_buf()
        } else {
            std::env::current_dir()?.join(repo_path)
        };
        return browse_repo(&repo_path, &selector, cd_file, post_cmd_file, &config);
    }

    // Decide starting view: CLI flag > config default > Workspace.
    let mode = match select_by {
        Some(v) => views::ViewMode::from_flag(v, &config)?,
        None => views::ViewMode::from_flag(&config.default_select_by, &config)?,
    };

    views::run(mode, &selector, &config, workspace, cd_file, post_cmd_file)
}

/// Register repo if needed and enter session action loop.
pub(crate) fn browse_repo(
    repo_path: &Path,
    selector: &dyn InteractiveSelector,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    config: &config::model::EzConfig,
) -> Result<()> {
    let index = repo::store::load_index()?;
    let repo_entry = if let Some(entry) = index.find_by_path(repo_path) {
        entry.clone()
    } else {
        repo::add_repo(Some(repo_path))?;
        let index = repo::store::load_index()?;
        index
            .find_by_path(repo_path)
            .cloned()
            .expect("just registered")
    };

    session_action_loop(&repo_entry, selector, cd_file, post_cmd_file, config)
}

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

/// Session selection loop with action keybinds.
pub(crate) fn session_action_loop(
    repo_entry: &repo::model::RepoEntry,
    selector: &dyn InteractiveSelector,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    config: &config::model::EzConfig,
) -> Result<()> {
    let keybinds = &config.keybinds;
    let plugin_views = plugin::collect_plugin_views("session", config).unwrap_or_default();
    let plugin_binds = plugin::collect_plugin_binds("session", config).unwrap_or_default();

    loop {
        let tree = session::ensure_default_session(&repo_entry.id, &repo_entry.path)?;
        let rendered = tree.render_tree();

        let session_items: Vec<SelectItem> = rendered
            .iter()
            .map(|(depth, s)| {
                let indent = "  ".repeat(*depth);
                let marker = if s.is_default {
                    " ★".yellow().to_string()
                } else {
                    String::new()
                };
                let path_info = s
                    .path
                    .as_ref()
                    .map(|p| format!(" → {}", p.display()).dimmed().to_string())
                    .unwrap_or_default();
                let labels = if s.labels.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", s.labels.join(",")).magenta().to_string()
                };
                SelectItem {
                    display: format!(
                        "{}{}{}{}{}",
                        indent,
                        s.name.bold().yellow(),
                        marker,
                        labels,
                        path_info
                    ),
                    value: s.id.clone(),
                }
            })
            .collect();

        let ez_bin = std::env::current_exe().ok();
        let repo_path_str = repo_entry.path.to_string_lossy();
        let preview_cmd = ez_bin.map(|bin| {
            format!(
                "{} preview --session-actions {}",
                bin.display(),
                repo_path_str
            )
        });

        let mut header = format!(
            "{}: new  {}: rename  {}: delete  {}: labels",
            keybinds.new_session,
            keybinds.rename_session,
            keybinds.delete_session,
            keybinds.edit_labels,
        );
        for pv in &plugin_views {
            header.push_str(&format!("  {}:{}", pv.key, pv.label));
        }
        for pb in &plugin_binds {
            header.push_str(&format!("  {}:{}", pb.key, pb.label));
        }

        let mut expect_keys: Vec<&str> = vec![
            keybinds.new_session.as_str(),
            keybinds.delete_session.as_str(),
            keybinds.rename_session.as_str(),
            keybinds.edit_labels.as_str(),
        ];
        for pv in &plugin_views {
            expect_keys.push(pv.key.as_str());
        }
        for pb in &plugin_binds {
            expect_keys.push(pb.key.as_str());
        }

        let action = selector.select_with_actions(
            &session_items,
            &repo_entry.name,
            preview_cmd.as_deref(),
            &expect_keys,
            Some(&header),
        )?;

        log::debug!(
            "session_action_loop: action={:?}",
            match &action {
                ActionResult::Select(i) => format!("Select({})", i),
                ActionResult::Action(k, i) => format!("Action({}, {})", k, i),
                ActionResult::Cancel => "Cancel".to_string(),
            }
        );

        match action {
            ActionResult::Select(idx) => {
                let selected = rendered[idx].1;
                let target_dir = selected
                    .path
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| repo_entry.path.clone());
                return write_cd_target(cd_file, &target_dir);
            }
            ActionResult::Action(key, idx) => {
                let selected = rendered[idx].1;
                match key.as_str() {
                    key if key == keybinds.new_session => {
                        let name = selector.input("Session name", None)?;
                        if !name.is_empty() {
                            session::create_child_session(
                                &repo_entry.id,
                                &selected.id,
                                &name,
                            )?;
                            eprintln!(
                                "{} {} → {}",
                                "Created:".green(),
                                name.bold(),
                                selected.name.dimmed()
                            );
                        }
                    }
                    key if key == keybinds.delete_session => {
                        let msg = format!("Delete session '{}'?", selected.name);
                        if selector.confirm(&msg, false)? {
                            session::delete_session_by_id(
                                &repo_entry.id,
                                &selected.id,
                                true,
                            )?;
                            eprintln!(
                                "{} {}",
                                "Deleted:".green(),
                                selected.name.bold()
                            );
                        }
                    }
                    key if key == keybinds.rename_session => {
                        let new_name = selector.input(
                            "New name",
                            Some(&selected.name),
                        )?;
                        if !new_name.is_empty() && new_name != selected.name {
                            session::rename_session_by_id(
                                &repo_entry.id,
                                &selected.id,
                                &new_name,
                            )?;
                            eprintln!(
                                "{} {} → {}",
                                "Renamed:".green(),
                                selected.name.bold(),
                                new_name.bold()
                            );
                        }
                    }
                    key if key == keybinds.edit_labels => {
                        let current = selected.labels.join(",");
                        let input = selector.input(
                            "Labels (comma-sep; prefix - to remove)",
                            Some(&current),
                        )?;
                        let (add, remove) = parse_label_input(&input);
                        let session_id = selected.id.clone();
                        let session_name = selected.name.clone();
                        let result =
                            session::set_session_labels(&repo_entry.id, &session_id, &add, &remove)?;
                        eprintln!(
                            "{} {} → {}",
                            "Labels on".green(),
                            session_name.bold(),
                            if result.is_empty() {
                                "(none)".dimmed().to_string()
                            } else {
                                result.join(", ").magenta().to_string()
                            }
                        );
                    }
                    _ => {
                        // Check plugin binds first (actions on selected session)
                        let mut handled = false;
                        for pb in &plugin_binds {
                            if key == pb.key {
                                let response = plugin::run_bind_hook(
                                    &pb.plugin_name,
                                    &pb.bind_name,
                                    &pb.key,
                                    "session",
                                    &selected.id,
                                    &selected.name,
                                    repo_entry,
                                    Some(selected),
                                    config,
                                )?;
                                if let Some(ref cd) = response.cd_target {
                                    write_cd_target(cd_file, cd)?;
                                }
                                if !response.post_shell_commands.is_empty() {
                                    write_post_commands(
                                        post_cmd_file,
                                        &response.post_shell_commands,
                                    )?;
                                }
                                if !response.shell_commands.is_empty() {
                                    plugin::runner::run_shell_commands(
                                        &response.shell_commands,
                                    )?;
                                }
                                if !response.post_shell_commands.is_empty()
                                    || response.cd_target.is_some()
                                {
                                    return Ok(());
                                }
                                handled = true;
                                break;
                            }
                        }
                        if handled {
                            continue;
                        }
                        // Check if it's a plugin view key
                        for pv in &plugin_views {
                            if key == pv.key {
                                views::run(
                                    views::ViewMode::Plugin {
                                        view_name: pv.view_name.clone(),
                                        plugin_name: pv.plugin_name.clone(),
                                    },
                                    selector,
                                    config,
                                    None,
                                    cd_file,
                                    post_cmd_file,
                                )?;
                                return Ok(());
                            }
                        }
                    }
                }
                // Loop back to show updated session list
            }
            ActionResult::Cancel => return Ok(()),
        }
    }
}

/// Drill into directories until a git repo is found or user selects one.
pub(crate) fn drill_into_directory(
    start: &Path,
    selector: &dyn InteractiveSelector,
) -> Result<Option<std::path::PathBuf>> {
    let mut current = start.to_path_buf();
    let mut history: Vec<std::path::PathBuf> = Vec::new();

    loop {
        if current.join(".git").exists() {
            return Ok(Some(current));
        }

        // Load once per level so registered-repo labels render consistently
        // with the Repo/Owner views.
        let index = repo::store::load_index().unwrap_or_default();
        let mut entries: Vec<(String, std::path::PathBuf)> = Vec::new();

        if let Ok(read_dir) = fs::read_dir(&current) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && !path
                        .file_name()
                        .map_or(true, |n| n.to_string_lossy().starts_with('.'))
                {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();

                    let display = if path.join(".git").exists() {
                        let branch = get_branch(&path).unwrap_or_else(|| "?".into());
                        let labels = index
                            .find_by_path(&path)
                            .and_then(|e| repo::store::load_repo_meta(&e.id).ok())
                            .map(|m| m.labels)
                            .unwrap_or_default();
                        format_repo_display(&name, None, Some(&branch), &labels)
                    } else {
                        name.bold().blue().to_string()
                    };

                    entries.push((display, path));
                }
            }
        }

        entries.sort_by(|a, b| a.0.cmp(&b.0));

        if entries.is_empty() {
            println!("{} {}", "No subdirectories in".yellow(), current.display());
            if let Some(prev) = history.pop() {
                current = prev;
                continue;
            }
            return Ok(None);
        }

        let items: Vec<SelectItem> = entries
            .iter()
            .map(|(display, path)| SelectItem {
                display: display.clone(),
                value: path.to_string_lossy().to_string(),
            })
            .collect();

        let ez_bin = std::env::current_exe().ok();
        let preview_cmd = ez_bin.map(|bin| format!("{} preview {{}}", bin.display()));

        let idx = match selector.select_one(
            &items,
            &current.file_name().unwrap_or_default().to_string_lossy(),
            preview_cmd.as_deref(),
        )? {
            Some(idx) => idx,
            None => {
                if let Some(prev) = history.pop() {
                    current = prev;
                    continue;
                }
                return Ok(None);
            }
        };

        history.push(current.clone());
        current = entries[idx].1.clone();
    }
}

/// Run a git command and capture stdout.
pub(crate) fn git_cmd(path: &Path, args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            } else {
                None
            }
        })
}

/// Get the current branch of a git repo.
pub(crate) fn get_branch(path: &Path) -> Option<String> {
    git_cmd(path, &["symbolic-ref", "--short", "HEAD"])
}

/// Shared display style for a repository row in any picker (drill-down,
/// repo view, owner view, etc.). `path` is the (collapse-tilded) path —
/// pass `None` when the surrounding context already shows it.
pub(crate) fn format_repo_display(
    name: &str,
    path: Option<&str>,
    branch: Option<&str>,
    labels: &[String],
) -> String {
    let mut parts = vec![name.bold().green().to_string()];
    if let Some(p) = path {
        parts.push(p.dimmed().to_string());
    }
    if let Some(b) = branch {
        parts.push(format!("[{b}]").cyan().to_string());
    }
    if !labels.is_empty() {
        parts.push(format!("[{}]", labels.join(",")).magenta().to_string());
    }
    parts.join(" ")
}

/// Parse a comma-separated label edit string.
///
/// - `foo, bar` → add `foo`, `bar`
/// - `-foo` → remove `foo`
///
/// Returns `(to_add, to_remove)`.
pub(crate) fn parse_label_input(input: &str) -> (Vec<String>, Vec<String>) {
    let mut add = Vec::new();
    let mut remove = Vec::new();
    for raw in input.split(',') {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(r) = token.strip_prefix('-') {
            let r = r.trim();
            if !r.is_empty() {
                remove.push(r.to_string());
            }
        } else {
            add.push(token.to_string());
        }
    }
    (add, remove)
}

#[cfg(test)]
mod tests {
    use super::parse_label_input;

    #[test]
    fn parses_add_and_remove() {
        let (a, r) = parse_label_input("foo, bar, -baz");
        assert_eq!(a, vec!["foo", "bar"]);
        assert_eq!(r, vec!["baz"]);
    }

    #[test]
    fn empty_input() {
        let (a, r) = parse_label_input("");
        assert!(a.is_empty());
        assert!(r.is_empty());
    }

    #[test]
    fn ignores_bare_dash() {
        let (a, r) = parse_label_input("-");
        assert!(a.is_empty());
        assert!(r.is_empty());
    }
}
