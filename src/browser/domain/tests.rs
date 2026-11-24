//! Unit tests for the domain layer (rows, preview, action handlers).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use console::strip_ansi_codes;

use crate::config::model::EzConfig;
use crate::error::{EzError, Result};
use crate::repo::model::{RepoEntry, RepoIndex};
use crate::session::model::{Session, SessionTree};
use crate::session::{UnmanagedWorktree, WorktreeInfo};

use super::super::git::BranchCache;
use super::super::selector::{ActionResult, InteractiveSelector, SelectItem, StageOutcome};
use super::super::SortMode;
use super::actions::{delete_unmanaged, handle_session_action, ActionCtx};
use super::preview::preview;
use super::rows::{
    directory_rows, label_rows, owner_rows, repo_view_rows, tree_rows, workspace_root_rows,
};
use super::session_rows::session_rows;
use super::{Effect, RowKind};

fn plain(s: &str) -> String {
    strip_ansi_codes(s).to_string()
}

fn fake_repo(id: &str, path: &Path) -> RepoEntry {
    RepoEntry {
        id: id.to_string(),
        path: path.to_path_buf(),
        name: path.file_name().unwrap().to_string_lossy().to_string(),
        registered_at: Utc::now(),
        is_git: true,
    }
}

fn fake_session(id: &str, name: &str, parent: Option<&str>) -> Session {
    Session {
        id: id.to_string(),
        name: name.to_string(),
        parent_id: parent.map(|p| p.to_string()),
        path: None,
        env: HashMap::new(),
        plugin_state: HashMap::new(),
        labels: Vec::new(),
        created_at: Utc::now(),
        is_default: false,
        bare: false,
        last_accessed: None,
    }
}

/// `<tmp>/a` and `<tmp>/b` are git-looking dirs, `<tmp>/plain` is not, `<tmp>/.hidden` is skipped.
fn workspace_fixture() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    for name in ["a", "b"] {
        fs::create_dir_all(tmp.path().join(name).join(".git")).unwrap();
    }
    fs::create_dir_all(tmp.path().join("plain")).unwrap();
    fs::create_dir_all(tmp.path().join(".hidden").join(".git")).unwrap();
    fs::write(tmp.path().join("file.txt"), "x").unwrap();
    tmp
}

#[test]
fn tree_rows_lists_root_and_repos_with_connectors() {
    let tmp = workspace_fixture();
    let config = EzConfig {
        workspace_roots: vec![tmp.path().to_string_lossy().to_string()],
        ..EzConfig::default()
    };
    let rows = tree_rows(&config, &RepoIndex::default(), &BranchCache::new());

    assert_eq!(rows.len(), 3);
    assert!(matches!(rows[0].kind, RowKind::Root(_)));
    assert!(plain(&rows[0].display).starts_with("└── "));
    assert_eq!(rows[0].value, tmp.path().to_string_lossy());

    let first = plain(&rows[1].display);
    let second = plain(&rows[2].display);
    assert!(first.contains("├── a [?]"), "{first}");
    assert!(second.contains("└── b [?]"), "{second}");
    match &rows[1].kind {
        RowKind::Repo { path, entry } => {
            assert_eq!(path, &tmp.path().join("a"));
            assert!(entry.is_none());
        }
        other => panic!("unexpected kind {other:?}"),
    }
    assert_eq!(rows[1].value, tmp.path().join("a").to_string_lossy());
}

#[test]
fn directory_rows_skip_hidden_and_sort() {
    let tmp = workspace_fixture();
    let rows = directory_rows(tmp.path(), &RepoIndex::default(), &BranchCache::new());
    assert_eq!(rows.len(), 3);
    let kinds: Vec<_> = rows
        .iter()
        .map(|r| (plain(&r.display), matches!(r.kind, RowKind::Directory(_))))
        .collect();
    assert!(kinds.iter().any(|(d, dir)| d == "plain" && *dir));
    assert!(kinds.iter().any(|(d, dir)| d == "a [?]" && !*dir));
    assert!(rows.windows(2).all(|w| w[0].display <= w[1].display));
}

#[test]
fn workspace_root_rows_expand_roots() {
    let tmp = tempfile::tempdir().unwrap();
    let config = EzConfig {
        workspace_roots: vec![tmp.path().to_string_lossy().to_string()],
        ..EzConfig::default()
    };
    let rows = workspace_root_rows(&config);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].value, tmp.path().to_string_lossy());
    assert!(matches!(&rows[0].kind, RowKind::Root(p) if p == tmp.path()));
}

#[test]
fn repo_and_owner_rows_from_index() {
    let tmp = tempfile::tempdir().unwrap();
    let mut index = RepoIndex::default();
    for (id, owner, name) in [
        ("ez-test-x1", "alice", "one"),
        ("ez-test-x2", "alice", "two"),
        ("ez-test-x3", "bob", "three"),
    ] {
        let path = tmp.path().join(owner).join(name);
        fs::create_dir_all(path.join(".git")).unwrap();
        index.repos.push(fake_repo(id, &path));
    }

    let rows = repo_view_rows(&index, SortMode::Alpha, &BranchCache::new());
    assert_eq!(rows.len(), 3);
    assert!(plain(&rows[0].display).starts_with("one "));
    assert!(plain(&rows[0].display).contains("[?]"));
    assert!(matches!(&rows[0].kind, RowKind::Repo { entry: Some(e), .. } if e.id == "ez-test-x1"));

    // LRU with no recorded access keeps index order.
    let lru = repo_view_rows(&index, SortMode::Lru, &BranchCache::new());
    assert_eq!(
        lru.iter().map(|r| r.value.clone()).collect::<Vec<_>>(),
        rows.iter().map(|r| r.value.clone()).collect::<Vec<_>>()
    );

    let owners = owner_rows(&index);
    assert_eq!(owners.len(), 2);
    assert_eq!(owners[0].value, "alice");
    assert!(plain(&owners[0].display).contains("(2 repos)"));
    assert!(plain(&owners[1].display).contains("(1 repo)"));
    assert!(matches!(&owners[0].kind, RowKind::Owner { repos } if repos.len() == 2));
}

#[test]
fn label_rows_empty_without_labels() {
    let tmp = tempfile::tempdir().unwrap();
    let mut index = RepoIndex::default();
    index
        .repos
        .push(fake_repo("ez-test-nolabels", &tmp.path().join("r")));
    assert!(label_rows(&index).is_empty());
}

#[test]
fn session_rows_managed_then_unmanaged() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = fake_repo("ez-test-sessions", tmp.path());
    let mut main = fake_session("s1", "main", None);
    main.is_default = true;
    let mut child = fake_session("s2", "feature", Some("s1"));
    child.labels = vec!["wip".into()];
    let tree = SessionTree {
        sessions: vec![main, child],
    };
    let rendered = tree.render_tree();
    let info = WorktreeInfo {
        branches: HashMap::new(),
        unmanaged: vec![UnmanagedWorktree {
            path: PathBuf::from("/tmp/ez-test-wt"),
            branch: Some("topic".into()),
            detached: false,
        }],
    };
    let attached: HashSet<String> = HashSet::new();
    let rows = session_rows(&repo, &rendered, &info, &attached, &BranchCache::new());

    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].value, "s1");
    assert!(plain(&rows[0].display).contains("main"));
    assert!(plain(&rows[0].display).contains('★'));
    assert!(plain(&rows[1].display).contains("[wip]"));
    assert!(
        matches!(&rows[1].kind, RowKind::Session { target_dir, .. } if target_dir == &repo.path)
    );
    assert_eq!(rows[2].value, "__header__");
    assert!(matches!(rows[2].kind, RowKind::Header));
    assert_eq!(rows[3].value, "__unmanaged__:/tmp/ez-test-wt");
    assert!(
        matches!(&rows[3].kind, RowKind::UnmanagedWorktree { branch: Some(b), .. } if b == "topic")
    );
}

#[test]
fn preview_renders_directory_to_string() {
    let tmp = workspace_fixture();
    let out = plain(&preview(tmp.path(), false, None).unwrap());
    assert!(out.contains("Contents"), "{out}");
    assert!(out.contains("Repositories (2)"), "{out}");
    assert!(out.contains("Directories (1)"), "{out}");
    assert!(out.contains("1 file(s)"), "{out}");
    assert!(out.contains("Keybinds"), "{out}");
    assert!(out.ends_with('\n'));
}

#[test]
fn preview_missing_path_and_header_row() {
    let out = plain(&preview(Path::new("/nonexistent/ez-test"), false, None).unwrap());
    assert_eq!(out, "Path does not exist\n/nonexistent/ez-test\n");

    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(
        preview(tmp.path(), true, Some("__header__")).unwrap(),
        String::new()
    );
}

#[test]
fn preview_unmanaged_worktree_missing_path() {
    let tmp = tempfile::tempdir().unwrap();
    let sid = "__unmanaged__:/nonexistent/ez-test-wt";
    let out = plain(&preview(tmp.path(), true, Some(sid)).unwrap());
    assert!(out.contains("ez-test-wt (not registered)"), "{out}");
    assert!(out.contains("Worktree path does not exist"), "{out}");
}

// ---- action handlers ----

#[derive(Clone, Copy)]
enum Answer {
    No,
    Interrupt,
}

struct MockSelector {
    confirm: Answer,
    input: Answer,
}

impl MockSelector {
    fn new(confirm: Answer, input: Answer) -> Self {
        MockSelector { confirm, input }
    }
}

impl InteractiveSelector for MockSelector {
    fn select_one(&self, _: &[SelectItem], _: &str, _: Option<&str>) -> Result<Option<usize>> {
        Ok(None)
    }
    fn select_many(&self, _: &[SelectItem], _: &str) -> Result<Vec<usize>> {
        Ok(Vec::new())
    }
    fn input(&self, _: &str, _: Option<&str>) -> Result<String> {
        match self.input {
            Answer::Interrupt => Err(EzError::Interrupted),
            Answer::No => Ok(String::new()),
        }
    }
    fn confirm(&self, _: &str, _: bool) -> Result<bool> {
        match self.confirm {
            Answer::Interrupt => Err(EzError::Interrupted),
            Answer::No => Ok(false),
        }
    }
    fn select_with_back(
        &self,
        _: &str,
        _: &[SelectItem],
        _: bool,
        _: Option<&str>,
    ) -> Result<StageOutcome> {
        Ok(StageOutcome::Cancel)
    }
    fn input_with_back(
        &self,
        _: &str,
        _: Option<&str>,
        _: bool,
        _: Option<&str>,
    ) -> Result<StageOutcome> {
        Ok(StageOutcome::Cancel)
    }
    fn select_with_actions(
        &self,
        _: &[SelectItem],
        _: &str,
        _: Option<&str>,
        _: &[&str],
        _: Option<&str>,
    ) -> Result<ActionResult> {
        Ok(ActionResult::Cancel)
    }
}

fn with_ctx<T>(selector: &MockSelector, f: impl FnOnce(&ActionCtx<'_>) -> T) -> T {
    let repo = fake_repo("ez-test-actions", Path::new("/tmp/ez-test-repo"));
    let config = EzConfig::default();
    let ctx = ActionCtx {
        repo_entry: &repo,
        config: &config,
        selector,
        plugin_binds: &[],
        post_cmd_file: None,
    };
    f(&ctx)
}

#[test]
fn cd_key_emits_cd_then_exit() {
    let sel = MockSelector::new(Answer::No, Answer::No);
    with_ctx(&sel, |ctx| {
        let key = ctx.config.keybinds.cd_session.clone();
        let mut s = fake_session("s1", "main", None);
        let fallback = handle_session_action(ctx, &key, &s).unwrap().unwrap();
        assert_eq!(
            fallback,
            vec![Effect::Cd(PathBuf::from("/tmp/ez-test-repo")), Effect::Exit]
        );
        s.path = Some(PathBuf::from("/tmp/ez-test-wt"));
        let with_path = handle_session_action(ctx, &key, &s).unwrap().unwrap();
        assert_eq!(
            with_path,
            vec![Effect::Cd(PathBuf::from("/tmp/ez-test-wt")), Effect::Exit]
        );
    });
}

#[test]
fn note_open_key_emits_open_note() {
    let sel = MockSelector::new(Answer::No, Answer::No);
    with_ctx(&sel, |ctx| {
        let key = ctx.config.keybinds.note_open.clone();
        let s = fake_session("s1", "main", None);
        let effects = handle_session_action(ctx, &key, &s).unwrap().unwrap();
        assert_eq!(
            effects,
            vec![Effect::OpenNote {
                repo_id: "ez-test-actions".into(),
                session_id: "s1".into()
            }]
        );
    });
}

#[test]
fn unknown_key_is_unhandled() {
    let sel = MockSelector::new(Answer::No, Answer::No);
    with_ctx(&sel, |ctx| {
        let s = fake_session("s1", "main", None);
        assert!(handle_session_action(ctx, "ctrl-9", &s).unwrap().is_none());
    });
}

#[test]
fn rename_with_empty_or_same_name_is_noop() {
    let sel = MockSelector::new(Answer::No, Answer::No);
    with_ctx(&sel, |ctx| {
        let key = ctx.config.keybinds.rename_session.clone();
        let s = fake_session("s1", "main", None);
        let effects = handle_session_action(ctx, &key, &s).unwrap().unwrap();
        assert!(effects.is_empty());
    });
}

#[test]
fn ctrl_c_in_sub_flows_propagates() {
    let sel = MockSelector::new(Answer::Interrupt, Answer::Interrupt);
    with_ctx(&sel, |ctx| {
        let s = fake_session("s1", "main", None);
        let rename = ctx.config.keybinds.rename_session.clone();
        assert!(matches!(
            handle_session_action(ctx, &rename, &s),
            Err(EzError::Interrupted)
        ));
        let labels = ctx.config.keybinds.edit_labels.clone();
        assert!(matches!(
            handle_session_action(ctx, &labels, &s),
            Err(EzError::Interrupted)
        ));
        assert!(matches!(
            delete_unmanaged(ctx, Path::new("/nonexistent/ez-test-wt"), Some("b")),
            Err(EzError::Interrupted)
        ));
    });
}

#[test]
fn declined_unmanaged_delete_has_no_effects() {
    let sel = MockSelector::new(Answer::No, Answer::No);
    with_ctx(&sel, |ctx| {
        let effects = delete_unmanaged(ctx, Path::new("/nonexistent/ez-test-wt"), None).unwrap();
        assert!(effects.is_empty());
    });
}
