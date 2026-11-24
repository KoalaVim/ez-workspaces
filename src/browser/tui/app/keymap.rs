//! Key bindings of the browser: global keys plus the per-level action keys
//! (session actions and plugin binds, repo-view actions, clone).

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::browser::tui::keys::Key;
use crate::browser::tui::level::{Level, Source, ViewKind};
use crate::config::model::EzConfig;
use crate::plugin::{self, PluginBindInfo, PluginViewInfo};

/// Things a global key can trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    SwitchView(ViewKind),
    SortToggle,
    ViewAll,
    Redraw,
}

pub(super) fn build_keymap(config: &EzConfig) -> HashMap<Key, Action> {
    let b = &config.keybinds;
    let mut map = HashMap::new();
    let binds = [
        (&b.view_tree, Action::SwitchView(ViewKind::Tree)),
        (&b.view_workspace, Action::SwitchView(ViewKind::Workspace)),
        (&b.view_repo, Action::SwitchView(ViewKind::Repo)),
        (&b.view_owner, Action::SwitchView(ViewKind::Owner)),
        (&b.view_label, Action::SwitchView(ViewKind::Label)),
        (&b.sort_toggle, Action::SortToggle),
        (&b.view_all, Action::ViewAll),
    ];
    for (spec, action) in binds {
        match Key::parse(spec) {
            Ok(key) => {
                map.insert(key, action);
            }
            Err(e) => log::warn!("tui: skipping key binding: {e}"),
        }
    }
    for info in plugin_views(config) {
        match Key::parse(&info.key) {
            Ok(key) => {
                map.insert(key, Action::SwitchView(ViewKind::plugin(info)));
            }
            Err(e) => log::warn!("tui: skipping plugin view key: {e}"),
        }
    }
    map.insert(
        Key::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
        Action::Redraw,
    );
    map
}

/// Plugin views usable from any core view or from a plugin view (each once).
fn plugin_views(config: &EzConfig) -> Vec<PluginViewInfo> {
    let mut views: Vec<PluginViewInfo> = Vec::new();
    for context in ["tree", "workspace", "repo", "owner", "label", "plugin"] {
        for info in plugin::collect_plugin_views(context, config).unwrap_or_default() {
            if !views.contains(&info) {
                views.push(info);
            }
        }
    }
    views
}

/// Which family of row actions a level offers (the same key can mean
/// different things: `alt-d` deletes a session and removes a repo).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum RowCtx {
    Sessions,
    RepoView,
    Drill,
}

impl RowCtx {
    pub fn of(level: &Level) -> Option<Self> {
        match level.source {
            Source::Sessions(_) => Some(RowCtx::Sessions),
            Source::View(ViewKind::Repo) => Some(RowCtx::RepoView),
            Source::Drill(_) => Some(RowCtx::Drill),
            _ => None,
        }
    }
}

/// Parsed action keys per context; values are the original config strings the
/// domain handlers compare against.
pub(super) struct RowKeys {
    by_ctx: HashMap<RowCtx, Vec<(Key, String)>>,
}

impl RowKeys {
    pub fn new(config: &EzConfig, binds: &[PluginBindInfo]) -> Self {
        let b = &config.keybinds;
        let mut sessions: Vec<&str> = vec![
            &b.new_session,
            &b.new_bare_session,
            &b.session_from_dirty,
            &b.delete_session,
            &b.rename_session,
            &b.edit_labels,
            &b.cd_session,
            &b.note_open,
            &b.note_cd,
        ];
        sessions.extend(binds.iter().map(|pb| pb.key.as_str()));
        let specs = [
            (RowCtx::Sessions, sessions),
            (RowCtx::RepoView, vec![&*b.edit_labels, &*b.remove_repo]),
            (RowCtx::Drill, vec![&*b.clone_repo]),
        ];
        let mut by_ctx = HashMap::new();
        for (ctx, list) in specs {
            let mut parsed = Vec::new();
            for spec in list {
                match Key::parse(spec) {
                    // A plain printable key would shadow typing in the filter.
                    Ok(k) if matches!(k.code, KeyCode::Char(_)) && k.mods.is_empty() => {
                        log::warn!("tui: ignoring action key '{spec}': it would shadow typing")
                    }
                    Ok(k) => parsed.push((k, spec.to_string())),
                    Err(e) => log::warn!("tui: skipping action key: {e}"),
                }
            }
            by_ctx.insert(ctx, parsed);
        }
        Self { by_ctx }
    }

    /// The config string of the action bound to `key` in `ctx`.
    pub fn find(&self, ctx: RowCtx, key: &Key) -> Option<&str> {
        self.by_ctx
            .get(&ctx)?
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, spec)| spec.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bind(key: &str) -> PluginBindInfo {
        PluginBindInfo {
            bind_name: "b".into(),
            plugin_name: "p".into(),
            key: key.into(),
            label: "l".into(),
            description: None,
        }
    }

    #[test]
    fn same_key_resolves_per_context() {
        let config = EzConfig::default();
        let keys = RowKeys::new(&config, &[]);
        let alt_d = Key::parse("alt-d").unwrap();
        assert_eq!(
            keys.find(RowCtx::Sessions, &alt_d),
            Some(config.keybinds.delete_session.as_str())
        );
        assert_eq!(
            keys.find(RowCtx::RepoView, &alt_d),
            Some(config.keybinds.remove_repo.as_str())
        );
        assert_eq!(keys.find(RowCtx::Drill, &alt_d), None);
        let alt_a = Key::parse(&config.keybinds.clone_repo).unwrap();
        assert!(keys.find(RowCtx::Drill, &alt_a).is_some());
    }

    #[test]
    fn plugin_binds_are_session_keys_and_plain_chars_are_ignored() {
        let config = EzConfig::default();
        let keys = RowKeys::new(&config, &[bind("alt-t"), bind("x")]);
        assert_eq!(
            keys.find(RowCtx::Sessions, &Key::parse("alt-t").unwrap()),
            Some("alt-t")
        );
        assert_eq!(keys.find(RowCtx::Sessions, &Key::parse("x").unwrap()), None);
        assert_eq!(
            keys.find(RowCtx::RepoView, &Key::parse("alt-t").unwrap()),
            None
        );
    }
}
