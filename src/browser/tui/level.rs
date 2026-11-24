//! One level of the navigation stack: a filtered list plus how to load its
//! rows and which preview its rows get. Loading is pure (no UI state), so it
//! can run on a worker thread.

use std::path::PathBuf;
use std::sync::Mutex;

use super::list::FilteredList;
use crate::browser::domain::rows;
use crate::browser::domain::session_rows::session_rows;
use crate::browser::domain::Row;
use crate::browser::session_loop::touch_last_accessed;
use crate::browser::{resolve_repo_entry, BranchCache, SortMode};
use crate::config::model::EzConfig;
use crate::error::{EzError, Result};
use crate::plugin::{self, PluginViewInfo};
use crate::repo;
use crate::repo::model::RepoEntry;
use crate::session;

/// The top-level views reachable with the view-switch keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ViewKind {
    Tree,
    Workspace,
    Repo,
    Owner,
    Label,
    /// A view provided by a plugin manifest `[[views]]` entry.
    Plugin(&'static PluginViewInfo),
}

/// Plugin views live for the whole process (a handful of tiny structs), which
/// keeps `ViewKind` `Copy` like the core views.
fn intern(info: PluginViewInfo) -> &'static PluginViewInfo {
    static KNOWN: Mutex<Vec<&'static PluginViewInfo>> = Mutex::new(Vec::new());
    let mut known = KNOWN.lock().unwrap();
    if let Some(found) = known.iter().find(|k| ***k == info) {
        return found;
    }
    let leaked: &'static PluginViewInfo = Box::leak(Box::new(info));
    known.push(leaked);
    leaked
}

impl ViewKind {
    /// A plugin view as a `ViewKind`.
    pub fn plugin(info: PluginViewInfo) -> Self {
        ViewKind::Plugin(intern(info))
    }
}

impl ViewKind {
    pub fn title(self) -> &'static str {
        match self {
            ViewKind::Tree => "tree",
            ViewKind::Workspace => "workspace",
            ViewKind::Repo => "repo",
            ViewKind::Owner => "owner",
            ViewKind::Label => "label",
            ViewKind::Plugin(info) => &info.view_name,
        }
    }

    /// Parse a `--select-by` / `default_select_by` value (core views and
    /// plugin view names, like the fzf browser).
    pub fn from_flag(v: &str, config: &EzConfig) -> Result<Self> {
        match v.to_ascii_lowercase().as_str() {
            "tree" | "t" => Ok(ViewKind::Tree),
            "workspace" | "ws" | "w" => Ok(ViewKind::Workspace),
            "repo" | "repos" | "r" => Ok(ViewKind::Repo),
            "owner" | "owners" | "o" => Ok(ViewKind::Owner),
            "label" | "labels" | "l" => Ok(ViewKind::Label),
            other => match plugin::find_plugin_view(other, config)? {
                Some(info) => Ok(ViewKind::plugin(info)),
                None => Err(EzError::Config(format!(
                    "unknown view '{other}' — expected one of: tree, workspace, repo, owner, label (or a plugin view)"
                ))),
            },
        }
    }
}

/// Where a level's rows come from.
#[derive(Clone, Debug)]
pub enum Source {
    View(ViewKind),
    /// Workspace directory drill-down.
    Drill(PathBuf),
    /// Repos of one owner.
    OwnerRepos(Vec<RepoEntry>),
    /// Rows handed over at creation (label children).
    Fixed,
    /// Session list of the repo at this path.
    Sessions(PathBuf),
}

/// Result of loading a level.
pub struct Loaded {
    pub rows: Vec<Row>,
    /// Resolved repo (session levels only).
    pub repo: Option<RepoEntry>,
    /// Preview command of a plugin view (`{}` stands for the highlighted value).
    pub preview_cmd: Option<String>,
}

/// Arguments of an in-process `ez preview` call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewReq {
    pub path: PathBuf,
    pub session_actions: bool,
    pub session_id: Option<String>,
    /// Plugin view preview: the shell command to run instead of a built-in preview.
    pub cmd: Option<String>,
}

impl PreviewReq {
    /// Cache key: identifies the rendered content.
    pub fn key(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.path.display(),
            self.session_actions,
            self.session_id.as_deref().unwrap_or(""),
            self.cmd.as_deref().unwrap_or("")
        )
    }
}

pub struct Level {
    /// Unique per level; worker results address levels by it.
    pub id: u64,
    /// Bumped on every load request; stale results are dropped.
    pub gen: u64,
    pub source: Source,
    pub title: String,
    pub list: FilteredList,
    pub sort: SortMode,
    pub loading: bool,
    pub error: Option<String>,
    pub repo: Option<RepoEntry>,
    /// Plugin views: preview command template.
    pub preview_cmd: Option<String>,
    /// The rows come from the snapshot; fresh rows have not arrived yet.
    pub stale: bool,
    /// A load request is in flight (`loading` is the visible subset of it).
    pub busy: bool,
    /// Branch data arrived while a load was in flight: reload once it ends.
    pub reload_again: bool,
    touched: bool,
}

impl Level {
    pub fn new(id: u64, source: Source, title: String, sort: SortMode) -> Self {
        Self {
            id,
            gen: 0,
            source,
            title,
            list: FilteredList::new(Vec::new()),
            sort,
            loading: false,
            error: None,
            repo: None,
            preview_cmd: None,
            stale: false,
            busy: false,
            reload_again: false,
            touched: false,
        }
    }

    /// Whether the sort-toggle key applies here.
    pub fn supports_sort(&self) -> bool {
        matches!(
            self.source,
            Source::View(ViewKind::Repo) | Source::Sessions(_)
        )
    }

    /// Whether this level lists the sessions of a repo.
    pub fn is_sessions(&self) -> bool {
        matches!(self.source, Source::Sessions(_))
    }

    /// First load of a session level touches the repo's `last_accessed`.
    pub fn take_touch(&mut self) -> bool {
        let first = self.is_sessions() && !self.touched;
        self.touched = true;
        first
    }

    /// Hint shown instead of an empty list.
    pub fn empty_hint(&self) -> String {
        match &self.source {
            Source::View(ViewKind::Tree) => "No repositories found in workspace roots.".into(),
            Source::View(ViewKind::Workspace) => {
                "No workspace roots configured. Add roots with: ez config --edit".into()
            }
            Source::View(ViewKind::Repo | ViewKind::Owner) => {
                "No registered repos. Use `ez add` or `ez clone`.".into()
            }
            Source::View(ViewKind::Label) => {
                "No labels found. Use `ez repo label add` or `ez session label add`.".into()
            }
            Source::View(ViewKind::Plugin(info)) => {
                format!("No items from plugin view '{}'.", info.view_name)
            }
            Source::Drill(_) => "No subdirectories.".into(),
            _ => "Nothing here.".into(),
        }
    }

    /// The preview the highlighted row gets; mirrors the fzf `--preview` per view.
    pub fn preview_req(&self) -> Option<PreviewReq> {
        let row = self.list.selected()?;
        match &self.source {
            Source::View(ViewKind::Tree | ViewKind::Repo)
            | Source::Drill(_)
            | Source::OwnerRepos(_) => Some(PreviewReq {
                path: PathBuf::from(&row.value),
                session_actions: false,
                session_id: None,
                cmd: None,
            }),
            Source::View(ViewKind::Plugin(_)) => {
                let template = self.preview_cmd.as_deref()?;
                Some(PreviewReq {
                    path: PathBuf::new(),
                    session_actions: false,
                    session_id: None,
                    // fzf hands the quoted first field to `{}`.
                    cmd: Some(template.replace("{}", &shell_quote(&row.value))),
                })
            }
            Source::Sessions(_) => Some(PreviewReq {
                path: self.repo.as_ref()?.path.clone(),
                session_actions: true,
                session_id: Some(row.value.clone()),
                cmd: None,
            }),
            _ => None,
        }
    }

    pub fn has_preview(&self) -> bool {
        match self.source {
            Source::View(ViewKind::Tree | ViewKind::Repo)
            | Source::Drill(_)
            | Source::OwnerRepos(_)
            | Source::Sessions(_) => true,
            Source::View(ViewKind::Plugin(_)) => self.preview_cmd.is_some(),
            _ => false,
        }
    }
}

/// Single-quote `s` for `sh`.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Build the rows of a level. Runs on a worker thread.
pub fn load(
    source: &Source,
    sort: SortMode,
    touch: bool,
    config: &EzConfig,
    cache: &BranchCache,
) -> Result<Loaded> {
    let plain = |rows| Loaded {
        rows,
        repo: None,
        preview_cmd: None,
    };
    match source {
        Source::View(ViewKind::Plugin(info)) => {
            let response = plugin::run_view_hook(&info.plugin_name, &info.view_name, config)?;
            let items = response.view_items.unwrap_or_default();
            Ok(Loaded {
                rows: rows::plugin_item_rows(&items),
                repo: None,
                preview_cmd: response.view_preview_cmd,
            })
        }
        Source::View(view) => {
            let index = repo::store::load_index()?;
            Ok(plain(match view {
                ViewKind::Tree => rows::tree_rows(config, &index, cache),
                ViewKind::Workspace => rows::workspace_root_rows(config),
                ViewKind::Repo => rows::repo_view_rows(&index, sort, cache),
                ViewKind::Owner => rows::owner_rows(&index),
                ViewKind::Label => rows::label_rows(&index),
                ViewKind::Plugin(_) => Vec::new(),
            }))
        }
        Source::Drill(dir) => {
            let index = repo::store::load_index().unwrap_or_default();
            Ok(plain(rows::directory_rows(dir, &index, cache)))
        }
        Source::OwnerRepos(entries) => {
            let refs: Vec<&RepoEntry> = entries.iter().collect();
            Ok(plain(rows::repo_rows(&refs, cache)))
        }
        Source::Fixed => Ok(plain(Vec::new())),
        Source::Sessions(path) => load_sessions(path, sort, touch, config, cache),
    }
}

/// Mirrors the per-iteration work of the fzf `session_action_loop`.
fn load_sessions(
    path: &std::path::Path,
    sort: SortMode,
    touch: bool,
    config: &EzConfig,
    cache: &BranchCache,
) -> Result<Loaded> {
    let entry = resolve_repo_entry(path)?;
    if touch {
        touch_last_accessed(&entry);
    }
    let tree = session::ensure_default_session(&entry.id, &entry.path)?;
    let worktree_info = session::build_worktree_info(&entry, &tree);
    let rendered = match sort {
        SortMode::Lru => tree.render_tree_lru(),
        SortMode::Alpha => tree.render_tree(),
    };
    let meta = repo::store::load_repo_meta(&entry.id).unwrap_or_default();
    let attached = plugin::get_attached_sessions(&entry, &meta, &tree, config);
    let rows = session_rows(&entry, &rendered, &worktree_info, &attached, cache);
    Ok(Loaded {
        rows,
        repo: Some(entry),
        preview_cmd: None,
    })
}
