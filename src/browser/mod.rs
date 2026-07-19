pub mod preview;
pub mod selector;
pub mod views;

use std::fs;
use std::path::{Path, PathBuf};

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::plugin;
use crate::repo;
use crate::session;
use crate::session::tree::format_session_tree_line;
use selector::{ActionResult, FzfSelector, InteractiveSelector, SelectItem};

pub use preview::preview;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SortMode {
    Alpha,
    Lru,
}

impl SortMode {
    pub fn toggle(self) -> Self {
        match self {
            SortMode::Alpha => SortMode::Lru,
            SortMode::Lru => SortMode::Alpha,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SortMode::Alpha => "A-Z",
            SortMode::Lru => "LRU",
        }
    }

    pub fn from_config(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "lru" => SortMode::Lru,
            _ => SortMode::Alpha,
        }
    }
}

pub struct BrowseOptions<'a> {
    pub cd_file: Option<&'a Path>,
    pub post_cmd_file: Option<&'a Path>,
    pub workspace: Option<&'a str>,
    pub repo_flag: Option<&'a Path>,
    pub select_by: Option<&'a str>,
    pub all: bool,
    pub on_enter: Option<&'a str>,
    pub on_create: Option<&'a str>,
}

/// Main interactive browser entry point (bare `ez` command).
pub fn browse(options: BrowseOptions<'_>) -> Result<()> {
    let BrowseOptions {
        cd_file,
        post_cmd_file,
        workspace,
        repo_flag,
        select_by,
        all,
        on_enter,
        on_create,
    } = options;
    let mut config = config::load()?;
    if let Some(v) = on_enter {
        config.on_enter = v.into();
    }
    if let Some(v) = on_create {
        config.on_create = v.into();
    }
    let selector = FzfSelector::new(&config.fzf)?;

    // --repo: jump straight to session picker for a specific repo
    if let Some(repo_path) = repo_flag {
        let repo_path = if repo_path.is_absolute() {
            repo_path.to_path_buf()
        } else {
            std::env::current_dir()?.join(repo_path)
        };
        browse_repo(&repo_path, &selector, cd_file, post_cmd_file, &config)?;
        return Ok(());
    }

    // Auto-detect: if cwd is inside a registered repo (or one of its worktrees),
    // jump straight to that repo's session picker. Skipped with --all.
    if !all {
        if let Some(repo_root) = detect_repo_root() {
            let index = repo::store::load_index()?;
            if let Some(entry) = index.find_by_path(&repo_root) {
                session_action_loop(entry, &selector, cd_file, post_cmd_file, &config)?;
                return Ok(());
            }
        }
    }

    // Decide starting view: CLI flag > config default > Workspace.
    let mode = match select_by {
        Some(v) => views::ViewMode::from_flag(v, &config)?,
        None => views::ViewMode::from_flag(&config.default_select_by, &config)?,
    };

    views::run(mode, &selector, &config, workspace, cd_file, post_cmd_file)
}

/// Register repo if needed and enter session action loop.
/// Returns `true` when a session was accepted (caller should exit the browser),
/// `false` when the user cancelled (caller should go back).
pub(crate) fn browse_repo(
    repo_path: &Path,
    selector: &dyn InteractiveSelector,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    config: &config::model::EzConfig,
) -> Result<bool> {
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

/// Returns true if the `on_create` value means "do nothing" (skip navigation after create).
pub(crate) fn on_create_is_noop(v: &str) -> bool {
    v.is_empty() || v == "none"
}

/// Apply a plugin HookResponse's side-effects (cd target, post-exit commands, inline commands).
fn apply_bind_response(
    response: &plugin::protocol::HookResponse,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
) -> Result<()> {
    if let Some(ref cd) = response.cd_target {
        write_cd_target(cd_file, cd)?;
    }
    if !response.post_shell_commands.is_empty() {
        if post_cmd_file.is_some() {
            write_post_commands(post_cmd_file, &response.post_shell_commands)?;
        } else {
            eprintln!("warning: shell wrapper is outdated. Re-run: eval \"$(ez init-shell zsh)\"");
            plugin::runner::run_shell_commands(&response.post_shell_commands)?;
        }
    }
    if !response.shell_commands.is_empty() {
        plugin::runner::run_shell_commands(&response.shell_commands)?;
    }
    Ok(())
}

/// Accept a session: either cd into it (default) or run a named plugin bind's hook.
///
/// The `on_enter` value is matched against each session-context bind's `label`, `bind_name`,
/// and `plugin_name` (in that order). On a match the bind's hook is invoked; if the response
/// carries a `cd_target` or `post_shell_commands` the function returns `Ok(())` after applying
/// them. When there is no match, or the bind produces no navigation effect, the function falls
/// back to a plain `cd` into `target_dir`.
pub(crate) fn accept_session(
    on_enter: &str,
    repo_entry: &repo::model::RepoEntry,
    selected: &session::model::Session,
    target_dir: &std::path::Path,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    config: &config::model::EzConfig,
) -> Result<()> {
    if on_enter == "cd" {
        return write_cd_target(cd_file, target_dir);
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
                let has_effect =
                    response.cd_target.is_some() || !response.post_shell_commands.is_empty();
                if has_effect {
                    apply_bind_response(&response, cd_file, post_cmd_file)?;
                    return Ok(());
                }
                // Plugin produced no navigation effect — fall through to cd.
            }
            Err(e) => {
                log::debug!("on_enter bind hook failed, falling back to cd: {e}");
                // Fall through to cd.
            }
        }
    } else {
        log::debug!("on_enter=\"{on_enter}\": no matching session bind found, falling back to cd");
    }

    write_cd_target(cd_file, target_dir)
}

/// Session selection loop with action keybinds.
/// Returns `true` when a session was accepted, `false` when cancelled.
pub(crate) fn session_action_loop(
    repo_entry: &repo::model::RepoEntry,
    selector: &dyn InteractiveSelector,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    config: &config::model::EzConfig,
) -> Result<bool> {
    // Update repo last_accessed timestamp on browse-into
    if let Ok(mut meta) = repo::store::load_repo_meta(&repo_entry.id) {
        meta.last_accessed = Some(chrono::Utc::now().to_rfc3339());
        let _ = repo::store::save_repo_meta(&repo_entry.id, &meta);
        log::debug!(
            "session_action_loop: updated last_accessed for repo '{}'",
            repo_entry.id
        );
    }

    let keybinds = &config.keybinds;
    let plugin_views = plugin::collect_plugin_views("session", config).unwrap_or_default();
    let plugin_binds = plugin::collect_plugin_binds("session", config).unwrap_or_default();
    let mut sort_mode = SortMode::from_config(&config.default_sort);

    loop {
        let tree = session::ensure_default_session(&repo_entry.id, &repo_entry.path)?;
        let rendered = match sort_mode {
            SortMode::Lru => tree.render_tree_lru(),
            SortMode::Alpha => tree.render_tree(),
        };

        let session_items: Vec<SelectItem> = rendered
            .iter()
            .map(|node| {
                let prefix = format_session_tree_line(node).dimmed().to_string();
                let marker = if node.session.is_default {
                    " ★".yellow().to_string()
                } else {
                    String::new()
                };
                let path_info = node
                    .session
                    .path
                    .as_ref()
                    .map(|p| format!(" → {}", p.display()).dimmed().to_string())
                    .unwrap_or_default();
                let bare_indicator = if node.session.bare {
                    " [bare]".dimmed().to_string()
                } else {
                    String::new()
                };
                let labels = if node.session.labels.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", node.session.labels.join(","))
                        .magenta()
                        .to_string()
                };
                let pr_indicator = format_pr_indicator(&node.session.env);
                let last_used = node
                    .session
                    .last_accessed
                    .as_ref()
                    .map(|ts| {
                        format!(" ({})", format_last_accessed(ts))
                            .dimmed()
                            .to_string()
                    })
                    .unwrap_or_default();
                SelectItem {
                    display: format!(
                        "{}{}{}{}{}{}{}{}",
                        prefix,
                        node.session.name.bold().yellow(),
                        marker,
                        bare_indicator,
                        pr_indicator,
                        labels,
                        last_used,
                        path_info
                    ),
                    value: node.session.id.clone(),
                }
            })
            .collect();

        let ez_bin = std::env::current_exe().ok();
        let repo_path_str = repo_entry.path.to_string_lossy();
        let preview_cmd = ez_bin.map(|bin| {
            format!(
                "{} preview --session-actions --session-id {{}} {}",
                bin.display(),
                repo_path_str
            )
        });

        let header = format!("sort: {} ({})", sort_mode.label(), keybinds.sort_toggle);

        let mut expect_keys: Vec<&str> = vec![
            keybinds.new_session.as_str(),
            keybinds.new_bare_session.as_str(),
            keybinds.session_from_dirty.as_str(),
            keybinds.delete_session.as_str(),
            keybinds.rename_session.as_str(),
            keybinds.edit_labels.as_str(),
            keybinds.cd_session.as_str(),
            keybinds.sort_toggle.as_str(),
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
                let selected = rendered[idx].session;
                update_last_accessed(repo_entry, &selected.id);
                let target_dir = selected
                    .path
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| repo_entry.path.clone());
                accept_session(
                    &config.on_enter,
                    repo_entry,
                    selected,
                    &target_dir,
                    cd_file,
                    post_cmd_file,
                    config,
                )?;
                return Ok(true);
            }
            ActionResult::Action(key, idx) => {
                let selected = rendered[idx].session;
                match key.as_str() {
                    key if key == keybinds.new_session => {
                        match session::name_builder::prompt_session_name(selector, config)? {
                            session::name_builder::NamePromptResult::Done { name, pr_metadata } => {
                                let env = pr_metadata
                                    .as_ref()
                                    .map(|pr| pr.to_session_env())
                                    .unwrap_or_default();
                                let created = session::create_child_session(
                                    &repo_entry.id,
                                    &selected.id,
                                    &name,
                                    false,
                                    env,
                                )?;
                                if let Some(pr) = &pr_metadata {
                                    if let Some(path) = &created.path {
                                        pr_merge_base_reset(path, &pr.base_ref);
                                    }
                                }
                                if on_create_is_noop(&config.on_create) {
                                    eprintln!(
                                        "{} {} → {}",
                                        "Created:".green(),
                                        name.bold(),
                                        selected.name.dimmed()
                                    );
                                } else {
                                    let target_dir = created
                                        .path
                                        .as_ref()
                                        .cloned()
                                        .unwrap_or_else(|| repo_entry.path.clone());
                                    accept_session(
                                        &config.on_create,
                                        repo_entry,
                                        &created,
                                        &target_dir,
                                        cd_file,
                                        post_cmd_file,
                                        config,
                                    )?;
                                    return Ok(true);
                                }
                            }
                            session::name_builder::NamePromptResult::Cancelled => {}
                        }
                    }
                    key if key == keybinds.new_bare_session => {
                        match session::name_builder::prompt_session_name(selector, config)? {
                            session::name_builder::NamePromptResult::Done { name, .. } => {
                                let created = session::create_child_session(
                                    &repo_entry.id,
                                    &selected.id,
                                    &name,
                                    true,
                                    std::collections::HashMap::new(),
                                )?;
                                if on_create_is_noop(&config.on_create) {
                                    eprintln!(
                                        "{} {} {} → {}",
                                        "Created bare:".green(),
                                        name.bold(),
                                        "[bare]".dimmed(),
                                        selected.name.dimmed()
                                    );
                                } else {
                                    let target_dir = created
                                        .path
                                        .as_ref()
                                        .cloned()
                                        .unwrap_or_else(|| repo_entry.path.clone());
                                    accept_session(
                                        &config.on_create,
                                        repo_entry,
                                        &created,
                                        &target_dir,
                                        cd_file,
                                        post_cmd_file,
                                        config,
                                    )?;
                                    return Ok(true);
                                }
                            }
                            session::name_builder::NamePromptResult::Cancelled => {}
                        }
                    }
                    key if key == keybinds.session_from_dirty => {
                        match session::name_builder::prompt_session_name(selector, config)? {
                            session::name_builder::NamePromptResult::Done { name, .. } => {
                                match session::from_dirty::session_from_dirty_inner(
                                    &name, None, None,
                                ) {
                                    Ok(created) => {
                                        if on_create_is_noop(&config.on_create) {
                                            eprintln!(
                                                "{} {} → {}",
                                                "Created from dirty:".green(),
                                                name.bold(),
                                                selected.name.dimmed()
                                            );
                                        } else {
                                            let target_dir = created
                                                .path
                                                .as_ref()
                                                .cloned()
                                                .unwrap_or_else(|| repo_entry.path.clone());
                                            accept_session(
                                                &config.on_create,
                                                repo_entry,
                                                &created,
                                                &target_dir,
                                                cd_file,
                                                post_cmd_file,
                                                config,
                                            )?;
                                            return Ok(true);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("{} {}", "From dirty failed:".red(), e);
                                    }
                                }
                            }
                            session::name_builder::NamePromptResult::Cancelled => {}
                        }
                    }
                    key if key == keybinds.delete_session => {
                        let dirty = session::cascade_dirty(&repo_entry.id, &selected.id)?;
                        let msg = if dirty.is_empty() {
                            format!("Delete session '{}'?", selected.name)
                        } else {
                            format!(
                                "Worktree(s) {:?} have uncommitted changes. Delete anyway?",
                                dirty
                            )
                        };
                        if selector.confirm(&msg, false)? {
                            session::delete_session_by_id(&repo_entry.id, &selected.id, true)?;
                            eprintln!("{} {}", "Deleted:".green(), selected.name.bold());
                        }
                    }
                    key if key == keybinds.rename_session => {
                        let new_name = selector.input("New name", Some(&selected.name))?;
                        if !new_name.is_empty() && new_name != selected.name {
                            session::rename_session_by_id(&repo_entry.id, &selected.id, &new_name)?;
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
                        let input = selector
                            .input("Labels (comma-sep; prefix - to remove)", Some(&current))?;
                        let (add, remove) = parse_label_input(&input);
                        let session_id = selected.id.clone();
                        let session_name = selected.name.clone();
                        let result = session::set_session_labels(
                            &repo_entry.id,
                            &session_id,
                            &add,
                            &remove,
                        )?;
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
                    key if key == keybinds.cd_session => {
                        let target_dir = selected
                            .path
                            .as_ref()
                            .cloned()
                            .unwrap_or_else(|| repo_entry.path.clone());
                        write_cd_target(cd_file, &target_dir)?;
                        return Ok(true);
                    }
                    key if key == keybinds.sort_toggle => {
                        sort_mode = sort_mode.toggle();
                        log::debug!("session_action_loop: sort toggled to {:?}", sort_mode);
                        continue;
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
                                apply_bind_response(&response, cd_file, post_cmd_file)?;
                                if response.cd_target.is_some()
                                    || !response.post_shell_commands.is_empty()
                                {
                                    return Ok(true);
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
                                return Ok(true);
                            }
                        }
                    }
                }
                // Loop back to show updated session list
            }
            ActionResult::Cancel => return Ok(false),
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
                        .is_none_or(|n| n.to_string_lossy().starts_with('.'))
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

/// Update `last_accessed` timestamp on a session and its repo after browser selection.
/// Also triggers PR auto-detection if the session has no PR metadata.
fn update_last_accessed(repo_entry: &repo::model::RepoEntry, session_id: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    if let Ok(mut tree) = session::store::load_sessions(&repo_entry.id) {
        session::detect_pr_for_session(&mut tree, session_id, repo_entry);
        if let Some(s) = tree.sessions.iter_mut().find(|s| s.id == session_id) {
            s.last_accessed = Some(now.clone());
        }
        let _ = session::store::save_sessions(&repo_entry.id, &tree);
    }
    if let Ok(mut meta) = repo::store::load_repo_meta(&repo_entry.id) {
        meta.last_accessed = Some(now);
        let _ = repo::store::save_repo_meta(&repo_entry.id, &meta);
    }
}

/// Format an ISO 8601 timestamp for display (e.g. "2h ago", "3d ago").
pub(crate) fn format_last_accessed(ts: &str) -> String {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) else {
        return ts.to_string();
    };
    let elapsed = chrono::Utc::now().signed_duration_since(dt);
    if elapsed.num_minutes() < 1 {
        format!("{}s ago", elapsed.num_seconds().max(0))
    } else if elapsed.num_hours() < 1 {
        format!("{}m ago", elapsed.num_minutes())
    } else if elapsed.num_hours() < 24 {
        format!("{}h ago", elapsed.num_hours())
    } else {
        format!("{}d ago", elapsed.num_days())
    }
}

/// After creating a PR-based session, reset the worktree to the merge-base
/// so that the PR's changes appear as dirty (unstaged) files.
pub(crate) fn pr_merge_base_reset(worktree_path: &Path, base_ref: &str) {
    log::debug!(
        "pr_merge_base_reset: resetting to merge-base with origin/{base_ref} in {}",
        worktree_path.display()
    );

    eprint!("{}", "Fetching base branch...".dimmed());
    let fetch_ok = git_run(worktree_path, &["fetch", "origin", base_ref]);
    eprint!("\r{}\r", " ".repeat(30));

    if !fetch_ok {
        eprintln!(
            "{}",
            format!("Warning: failed to fetch origin/{base_ref}").yellow()
        );
        return;
    }

    let base_remote = format!("origin/{base_ref}");
    let merge_base = match git_cmd(worktree_path, &["merge-base", "HEAD", &base_remote]) {
        Some(mb) => mb,
        None => {
            eprintln!(
                "{}",
                "Warning: could not determine merge-base, worktree has PR branch checked out normally"
                    .yellow()
            );
            return;
        }
    };

    log::debug!("pr_merge_base_reset: merge-base={merge_base}");

    if git_run(worktree_path, &["reset", "--mixed", &merge_base]) {
        eprintln!("{}", "PR changes shown as dirty files.".green());
    } else {
        eprintln!(
            "{}",
            "Warning: git reset failed, worktree has PR branch checked out normally".yellow()
        );
    }
}

/// Format a PR status indicator for display in the session picker.
pub(crate) fn format_pr_indicator(env: &std::collections::HashMap<String, String>) -> String {
    match (env.get("ez_pr_number"), env.get("ez_pr_status")) {
        (Some(num), Some(status)) => {
            let indicator = format!("[PR #{num} {status}]");
            match status.as_str() {
                "merged" => format!(" {}", indicator.magenta()),
                "closed" => format!(" {}", indicator.red()),
                _ => format!(" {}", indicator.green()),
            }
        }
        (Some(num), None) => format!(" {}", format!("[PR #{num}]").green()),
        _ => String::new(),
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
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            } else {
                None
            }
        })
}

/// Get the current branch of a git repo.
pub(crate) fn get_branch(path: &Path) -> Option<String> {
    git_cmd(path, &["symbolic-ref", "--short", "HEAD"])
}

/// True if the git worktree at `path` has uncommitted changes.
pub(crate) fn is_dirty(path: &Path) -> bool {
    git_cmd(path, &["status", "--porcelain"])
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

/// Run a git command, returning whether it exited successfully (output is discarded).
pub(crate) fn git_run(path: &Path, args: &[&str]) -> bool {
    std::process::Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// True if a local branch with this exact name exists in the repo at `path`.
pub(crate) fn branch_exists(path: &Path, name: &str) -> bool {
    git_run(
        path,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{name}"),
        ],
    )
}

/// Resolve the main repository root from the current directory.
/// Works from both the repo itself and any of its worktrees by using
/// `git rev-parse --git-common-dir` which always points to the main
/// repo's `.git` directory.
fn detect_repo_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let common_dir = git_cmd(&cwd, &["rev-parse", "--git-common-dir"])?;
    let common_path = PathBuf::from(&common_dir);
    let abs = if common_path.is_absolute() {
        common_path
    } else {
        cwd.join(&common_path)
    };
    // --git-common-dir returns the .git dir; the repo root is its parent
    abs.canonicalize().ok()?.parent().map(|p| p.to_path_buf())
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
