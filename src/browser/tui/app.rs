//! Application state of the native browser.
//!
//! A navigation stack of [`Level`]s (view -> drill-down / group -> session
//! list). Rows are built on worker threads and arrive over the results
//! channel, so the first frame never waits on disk or git. Top-level views
//! keep their level (query and cursor) while another view is shown.

mod actions;
mod draw;
mod handler;
mod keymap;
mod loader;
mod snapshot_io;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use self::keymap::{build_keymap, build_tabs, Action, RowKeys};
use self::loader::Loader;
use super::event::{self, Flow};
use super::keys::Key;
use super::level::{Level, Source, ViewKind};
use super::modal::{Core, NativeSelector};
use super::nav::Nav;
use super::preview_pane::PreviewPane;
use super::snapshot::Snapshot;
use super::terminal::SharedGuard;
use super::workers::Pool;
use crate::browser::domain::{Level as Severity, RowKind};
use crate::browser::selector::set_confirm_override;
use crate::browser::{BranchCache, SortMode};
use crate::config::model::EzConfig;
use crate::error::Result;
use crate::plugin::{self, PluginBindInfo};

pub use self::loader::Msg;

/// Where the browser starts.
pub struct Start {
    pub view: ViewKind,
    /// Directory to drill into below the view (`--workspace`).
    pub drill: Option<PathBuf>,
    /// Repo whose session list opens below the view (`--repo`, cwd repo).
    pub sessions: Option<PathBuf>,
}

/// Files the shell wrapper reads after ez exits.
pub struct Outputs {
    pub cd_file: Option<PathBuf>,
    pub post_cmd_file: Option<PathBuf>,
}

struct App {
    config: Arc<EzConfig>,
    loader: Loader,
    keymap: HashMap<Key, Action>,
    /// View tabs shown under the search bar.
    tabs: Vec<(ViewKind, String)>,
    nav: Nav<Level>,
    /// Top-level views that are not currently shown.
    saved: HashMap<ViewKind, Level>,
    next_id: u64,
    /// Bumped whenever rows change; invalidates cached previews.
    data_gen: u64,
    preview: PreviewPane,
    shared: SharedGuard,
    /// Terminal, stdout capture and frozen frame shared with modals.
    core: Rc<Core>,
    selector: Rc<NativeSelector>,
    row_keys: RowKeys,
    plugin_binds: Rc<Vec<PluginBindInfo>>,
    outputs: Outputs,
    /// Transient message line; cleared by the next key press.
    notice: Option<(String, Severity)>,
    /// Lines to print on stdout after the terminal is restored.
    after_exit: Vec<String>,
    /// Last run's rows (None when `[native] snapshot` is off).
    snapshot: Option<Snapshot>,
    snapshot_path: Option<PathBuf>,
    /// Branch lookups still running on workers.
    branch_inflight: usize,
    /// Branch lookups still running, per requesting level.
    branch_pending: HashMap<u64, usize>,
    /// Without a snapshot the first frame waits for fresh rows (at most
    /// [`FIRST_PAINT_WAIT`]) instead of showing an empty list.
    first_paint_at: Option<Instant>,
}

const FIRST_PAINT_WAIT: Duration = Duration::from_millis(1000);

/// Run the browser; returns the stdout lines to print once the terminal is restored.
pub fn run(
    guard: SharedGuard,
    config: &EzConfig,
    start: Start,
    outputs: Outputs,
) -> Result<Vec<String>> {
    let (tx, rx) = mpsc::channel();
    let mut app = App::new(Rc::clone(&guard), config.clone(), tx, start, outputs);
    let result = event::run(&guard, &mut app, &rx);
    set_confirm_override(None);
    app.save_snapshot();
    result.map(|()| std::mem::take(&mut app.after_exit))
}

impl App {
    fn new(
        shared: SharedGuard,
        config: EzConfig,
        tx: Sender<Msg>,
        start: Start,
        outputs: Outputs,
    ) -> Self {
        let keymap = build_keymap(&config);
        let plugin_binds = plugin::collect_plugin_binds("session", &config).unwrap_or_default();
        let row_keys = RowKeys::new(&config, &plugin_binds);
        let core = Rc::new(Core::new(Rc::clone(&shared)));
        let selector = Rc::new(NativeSelector::new(Rc::clone(&core)));
        // Code that asks for confirmation without a selector gets a modal too.
        let hook_selector = Rc::clone(&selector);
        set_confirm_override(Some(Rc::new(move |prompt, default| {
            use crate::browser::selector::InteractiveSelector as _;
            hook_selector.confirm(prompt, default)
        })));
        let config = Arc::new(config);
        let pool = Pool::new(tx.clone());
        let loader = Loader {
            config: Arc::clone(&config),
            cache: Arc::new(BranchCache::lazy()),
            pool: pool.clone(),
            tx,
        };
        let (snapshot, snapshot_path) = snapshot_io::load(&config);
        let sort = SortMode::from_config(&config.default_sort);
        let mut root = Level::new(0, Source::View(start.view), start.view.title().into(), sort);
        let seeded = snapshot.as_ref().is_some_and(|s| s.seed(&mut root));
        loader.request(&mut root);
        let tabs = build_tabs(&config);
        let mut app = Self {
            tabs,
            config,
            loader,
            keymap,
            nav: Nav::new(root),
            saved: HashMap::new(),
            next_id: 1,
            data_gen: 0,
            preview: PreviewPane::new(pool),
            shared,
            core,
            selector,
            row_keys,
            plugin_binds: Rc::new(plugin_binds),
            outputs,
            notice: None,
            after_exit: Vec::new(),
            snapshot,
            snapshot_path,
            branch_inflight: 0,
            branch_pending: HashMap::new(),
            first_paint_at: (!seeded).then(Instant::now),
        };
        if let Some(dir) = start.drill {
            app.open_dir(&dir);
        }
        if let Some(repo) = start.sessions {
            app.push_sessions(&repo);
        }
        app
    }

    fn new_level(&mut self, source: Source, title: String) -> Level {
        let id = self.next_id;
        self.next_id += 1;
        Level::new(
            id,
            source,
            title,
            SortMode::from_config(&self.config.default_sort),
        )
    }

    fn push_loading(&mut self, source: Source, title: String) {
        let mut level = self.new_level(source, title);
        if let Some(snapshot) = &self.snapshot {
            snapshot.seed(&mut level);
        }
        self.loader.request(&mut level);
        self.nav.push(level);
    }

    fn push_sessions(&mut self, repo_path: &Path) {
        let title = repo_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        self.push_loading(Source::Sessions(repo_path.to_path_buf()), title);
    }

    /// Workspace drill-down: a git repo opens its sessions, other directories list their children.
    fn open_dir(&mut self, dir: &Path) {
        if dir.join(".git").exists() {
            self.push_sessions(dir);
        } else {
            let title = dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| dir.display().to_string());
            self.push_loading(Source::Drill(dir.to_path_buf()), title);
        }
    }

    fn current_view(&self) -> ViewKind {
        match self.nav.root().source {
            Source::View(v) => v,
            _ => ViewKind::Tree,
        }
    }

    /// View keys also work in owner/label sub-lists (as in fzf mode).
    fn can_switch_view(&self) -> bool {
        self.nav.is_root() || matches!(self.nav.top().source, Source::OwnerRepos(_) | Source::Fixed)
    }

    fn switch_view(&mut self, view: ViewKind) {
        if !self.can_switch_view() || view == self.current_view() {
            return;
        }
        self.nav.truncate_to_root();
        let next = match self.saved.remove(&view) {
            Some(mut level) => {
                // Branch data arrived while the view was hidden.
                if std::mem::take(&mut level.reload_again) && !level.busy {
                    self.loader.request_quiet(&mut level);
                }
                level
            }
            None => {
                let mut level = self.new_level(Source::View(view), view.title().into());
                if let Some(snapshot) = &self.snapshot {
                    snapshot.seed(&mut level);
                }
                self.loader.request(&mut level);
                level
            }
        };
        let old = self.nav.replace_root(next);
        if let Source::View(kind) = old.source {
            self.saved.insert(kind, old);
        }
        log::debug!("tui: switched to {view:?}");
    }

    fn act(&mut self, action: Action) -> Flow {
        match action {
            Action::SwitchView(view) => self.switch_view(view),
            Action::SortToggle => {
                let loader = &self.loader;
                let level = self.nav.top_mut();
                if level.supports_sort() {
                    level.sort = level.sort.toggle();
                    log::debug!("tui: sort toggled to {:?}", level.sort);
                    loader.request(level);
                }
            }
            Action::ViewAll => {
                // fzf: leaves the session list for the global view.
                if self.nav.top().is_sessions() {
                    self.nav.back();
                }
            }
            Action::Redraw => return Flow::FullRedraw,
        }
        Flow::Continue
    }

    /// Enter on the highlighted row opens the next level (or accepts a session).
    fn enter(&mut self) -> Result<Flow> {
        let level = self.nav.top();
        let Some(row) = level.list.selected().cloned() else {
            return Ok(Flow::Continue);
        };
        let view = match level.source {
            Source::View(v) => Some(v),
            _ => None,
        };
        match row.kind {
            RowKind::Repo { path, .. } => {
                if path.exists() {
                    self.push_sessions(&path);
                } else {
                    self.refuse(&format!("Repo path {} no longer exists", path.display()));
                }
            }
            RowKind::Root(path) if view == Some(ViewKind::Workspace) => self.open_dir(&path),
            RowKind::Directory(path) => self.open_dir(&path),
            RowKind::Owner { repos } => {
                let source = Source::OwnerRepos(repos);
                self.push_loading(source, row.value);
            }
            RowKind::Label { children } => {
                let mut level = self.new_level(Source::Fixed, row.value);
                level.list.set_rows(children);
                self.nav.push(level);
            }
            RowKind::Session { .. } | RowKind::UnmanagedWorktree { .. } => {
                return self.accept_selected();
            }
            RowKind::PluginItem => {
                if let Some(ViewKind::Plugin(info)) = view {
                    return self.plugin_select(info, &row);
                }
            }
            RowKind::Root(_) | RowKind::Header | RowKind::Cached(_) => {}
        }
        Ok(Flow::Continue)
    }

    /// A branch lookup finished: rebuild the rows that showed a placeholder
    /// for it, without touching cached previews.
    /// A branch lookup finished. Once all lookups of a level are done, that
    /// level reloads once (instead of every level on every lookup).
    fn branch_resolved(&mut self, id: u64) {
        self.branch_inflight = self.branch_inflight.saturating_sub(1);
        let left = self.branch_pending.entry(id).or_default();
        *left = left.saturating_sub(1);
        log::debug!(
            "tui: branch resolved for level {id} ({} in flight)",
            self.branch_inflight
        );
        if *left > 0 {
            return;
        }
        self.branch_pending.remove(&id);
        let loader = &self.loader;
        if let Some(level) = self.nav.iter_mut().find(|l| l.id == id) {
            if level.busy {
                level.reload_again = true;
            } else {
                loader.request_quiet(level);
            }
        } else if let Some(level) = self.saved.values_mut().find(|l| l.id == id) {
            // Hidden views reload when they are shown again.
            level.reload_again = true;
        }
    }

    /// Reload every level (rows and previews) after data changed.
    fn refresh(&mut self) {
        self.data_gen += 1;
        let loader = &self.loader;
        for level in self.nav.iter_mut().chain(self.saved.values_mut()) {
            if !matches!(level.source, Source::Fixed) {
                loader.request(level);
            }
        }
        log::debug!("tui: refresh (data generation {})", self.data_gen);
    }

    /// Show `text` as a warning and reload: the target is gone or changed.
    fn refuse(&mut self, text: &str) {
        log::debug!("tui: refused action: {text}");
        self.notice = Some((text.to_string(), Severity::Warn));
        self.refresh();
    }
}

fn find_level<'a>(
    nav: &'a mut Nav<Level>,
    saved: &'a mut HashMap<ViewKind, Level>,
    id: u64,
) -> Option<&'a mut Level> {
    nav.iter_mut()
        .find(|l| l.id == id)
        .or_else(|| saved.values_mut().find(|l| l.id == id))
}
