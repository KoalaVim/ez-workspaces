//! fzf front end for the per-repo session list. Rows come from
//! `domain::session_rows`, key dispatch from `domain::actions`; this file only
//! drives the selector and applies the resulting effects.

use std::path::Path;

use crate::config;
use crate::error::Result;
use crate::plugin;
use crate::repo;
use crate::session;

use super::domain::actions::{
    delete_unmanaged, handle_session_action, select_session, select_unmanaged, ActionCtx,
};
use super::domain::session_rows::session_rows;
use super::domain::{to_items, RowKind};
use super::effects::{apply_effects, EffectTarget};
use super::git::BranchCache;
use super::selector::{ActionResult, InteractiveSelector};
use super::{views, SessionLoopResult, SortMode};

/// Session selection loop with action keybinds.
/// Returns `Accepted` when a session was accepted, `Cancelled` when cancelled,
/// or `ViewAll` when the user asked to switch to the global browser view.
pub(crate) fn session_action_loop(
    repo_entry: &repo::model::RepoEntry,
    selector: &dyn InteractiveSelector,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    config: &config::model::EzConfig,
    branch_cache: &BranchCache,
) -> Result<SessionLoopResult> {
    touch_last_accessed(repo_entry);

    let keybinds = &config.keybinds;
    let plugin_views = plugin::collect_plugin_views("session", config).unwrap_or_default();
    let plugin_binds = plugin::collect_plugin_binds("session", config).unwrap_or_default();
    let mut sort_mode = SortMode::from_config(&config.default_sort);

    let ctx = ActionCtx {
        repo_entry,
        config,
        selector,
        plugin_binds: &plugin_binds,
        post_cmd_file,
    };
    let target = EffectTarget {
        cd_file,
        post_cmd_file,
        config,
    };

    loop {
        let tree = session::ensure_default_session(&repo_entry.id, &repo_entry.path)?;
        let worktree_info = session::build_worktree_info(repo_entry, &tree);
        let rendered = match sort_mode {
            SortMode::Lru => tree.render_tree_lru(),
            SortMode::Alpha => tree.render_tree(),
        };

        let repo_meta = repo::store::load_repo_meta(&repo_entry.id).unwrap_or_default();
        let attached = plugin::get_attached_sessions(repo_entry, &repo_meta, &tree, config);

        let rows = session_rows(
            repo_entry,
            &rendered,
            &worktree_info,
            &attached,
            branch_cache,
        );
        let all_items = to_items(&rows);

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
            keybinds.note_open.as_str(),
            keybinds.note_cd.as_str(),
            keybinds.view_all.as_str(),
        ];
        for pv in &plugin_views {
            expect_keys.push(pv.key.as_str());
        }
        for pb in &plugin_binds {
            expect_keys.push(pb.key.as_str());
        }

        let action = selector.select_with_actions(
            &all_items,
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
                let effects = match &rows[idx].kind {
                    RowKind::Session { session, .. } => select_session(&ctx, session),
                    RowKind::UnmanagedWorktree { path, branch } => {
                        select_unmanaged(&ctx, path, branch.as_deref())
                    }
                    // Separator header: nothing to do.
                    _ => continue,
                };
                if apply_effects(&effects, &target)? {
                    return Ok(SessionLoopResult::Accepted);
                }
            }
            ActionResult::Action(key, idx) => {
                if key == keybinds.view_all {
                    return Ok(SessionLoopResult::ViewAll);
                }
                if key == keybinds.sort_toggle {
                    sort_mode = sort_mode.toggle();
                    log::debug!("session_action_loop: sort toggled to {:?}", sort_mode);
                    continue;
                }
                let effects = match &rows[idx].kind {
                    RowKind::Session { session, .. } => {
                        match handle_session_action(&ctx, &key, session)? {
                            Some(effects) => effects,
                            None => {
                                // Not a session action or bind: maybe a plugin view key.
                                if let Some(pv) = plugin_views.iter().find(|pv| key == pv.key) {
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
                                        branch_cache,
                                    )?;
                                    return Ok(SessionLoopResult::Accepted);
                                }
                                continue;
                            }
                        }
                    }
                    RowKind::UnmanagedWorktree { path, branch }
                        if key == keybinds.delete_session =>
                    {
                        delete_unmanaged(&ctx, path, branch.as_deref())?
                    }
                    _ => continue,
                };
                if apply_effects(&effects, &target)? {
                    return Ok(SessionLoopResult::Accepted);
                }
            }
            ActionResult::Cancel => return Ok(SessionLoopResult::Cancelled),
        }
        // Loop back to show updated session list
    }
}

/// Update the repo's `last_accessed` timestamp on browse-into.
pub(crate) fn touch_last_accessed(repo_entry: &repo::model::RepoEntry) {
    if let Ok(mut meta) = repo::store::load_repo_meta(&repo_entry.id) {
        meta.last_accessed = Some(chrono::Utc::now().to_rfc3339());
        let _ = repo::store::save_repo_meta(&repo_entry.id, &meta);
        log::debug!(
            "touch_last_accessed: updated last_accessed for repo '{}'",
            repo_entry.id
        );
    }
}
