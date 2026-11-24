//! Snapshot of the last shown rows (design D6) for an instant first frame.
//!
//! Written to `<data_dir>/ez/browser-snapshot.toml` on exit (temp file plus
//! rename, so a crash never leaves a half-written file) and read on start.
//! Rows are display-only: they carry no payload, so nothing can act on a
//! snapshot row before fresh rows replaced it. A missing, corrupt or
//! version-mismatched file is ignored (logged at debug level).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::level::{Level, Source};
use crate::browser::domain::{Row, RowKind};
use crate::error::Result;

/// Bump when the file layout changes; other versions are ignored.
pub const VERSION: u32 = 1;
/// Rows kept per list (a snapshot only has to fill the first screens).
const MAX_ROWS: usize = 500;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SnapRow {
    pub display: String,
    pub value: String,
    pub kind: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub version: u32,
    /// Title of the top-level view shown when the browser exited.
    pub view: String,
    /// Rows per top-level view, by view title.
    #[serde(default)]
    pub views: BTreeMap<String, Vec<SnapRow>>,
    /// Rows per session list, by repo path.
    #[serde(default)]
    pub sessions: BTreeMap<String, Vec<SnapRow>>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            version: VERSION,
            view: String::new(),
            views: BTreeMap::new(),
            sessions: BTreeMap::new(),
        }
    }
}

/// `<dir>/browser-snapshot.toml`
pub fn path_in(dir: &Path) -> PathBuf {
    dir.join("browser-snapshot.toml")
}

/// Read the snapshot at `path`; anything unusable yields `None`.
pub fn load_from(path: &Path) -> Option<Snapshot> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            log::debug!("snapshot: not read ({}): {e}", path.display());
            return None;
        }
    };
    match toml::from_str::<Snapshot>(&text) {
        Ok(snap) if snap.version == VERSION => {
            log::debug!(
                "snapshot: loaded {} views, {} session lists",
                snap.views.len(),
                snap.sessions.len()
            );
            Some(snap)
        }
        Ok(snap) => {
            log::debug!(
                "snapshot: ignoring version {} (want {VERSION})",
                snap.version
            );
            None
        }
        Err(e) => {
            log::debug!("snapshot: ignoring corrupt file {}: {e}", path.display());
            None
        }
    }
}

/// Write the snapshot atomically.
pub fn save_to(path: &Path, snapshot: &Snapshot) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = toml::to_string(snapshot)
        .map_err(|e| crate::error::EzError::Config(format!("snapshot: {e}")))?;
    let tmp = path.with_extension(format!("toml.tmp{}", std::process::id()));
    std::fs::write(&tmp, text)?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    log::debug!("snapshot: saved to {}", path.display());
    Ok(())
}

fn tag(kind: &RowKind) -> String {
    match kind {
        RowKind::Root(_) => "root",
        RowKind::Directory(_) => "directory",
        RowKind::Repo { .. } => "repo",
        RowKind::Session { .. } => "session",
        RowKind::UnmanagedWorktree { .. } => "unmanaged",
        RowKind::Header => "header",
        RowKind::Owner { .. } => "owner",
        RowKind::Label { .. } => "label",
        RowKind::PluginItem => "plugin",
        RowKind::Cached(tag) => tag,
    }
    .to_string()
}

fn snap_rows(rows: &[Row]) -> Vec<SnapRow> {
    rows.iter()
        .take(MAX_ROWS)
        .map(|r| SnapRow {
            display: r.display.clone(),
            value: r.value.clone(),
            kind: tag(&r.kind),
        })
        .collect()
}

fn to_rows(rows: &[SnapRow]) -> Vec<Row> {
    rows.iter()
        .map(|r| Row {
            display: r.display.clone(),
            value: r.value.clone(),
            kind: RowKind::Cached(r.kind.clone()),
        })
        .collect()
}

impl Snapshot {
    /// Rows saved for a level, if it is one the snapshot covers.
    fn rows_for(&self, source: &Source) -> Option<&Vec<SnapRow>> {
        match source {
            Source::View(view) => self.views.get(view.title()),
            Source::Sessions(path) => self.sessions.get(path.to_string_lossy().as_ref()),
            _ => None,
        }
    }

    /// Fill `level` with its saved rows (marked stale) when there are any.
    pub fn seed(&self, level: &mut Level) -> bool {
        let Some(saved) = self.rows_for(&level.source).filter(|r| !r.is_empty()) else {
            return false;
        };
        level.list.set_rows(to_rows(saved));
        level.stale = true;
        true
    }

    /// Record the rows of `level`; levels that are not loaded keep their old entry.
    pub fn record(&mut self, level: &Level) {
        if level.stale || level.busy || level.error.is_some() || level.gen == 0 {
            return;
        }
        let rows = snap_rows(level.list.rows());
        match &level.source {
            Source::View(view) => {
                self.views.insert(view.title().to_string(), rows);
            }
            Source::Sessions(path) => {
                self.sessions
                    .insert(path.to_string_lossy().to_string(), rows);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::tui::level::ViewKind;
    use crate::browser::SortMode;

    fn sample() -> Snapshot {
        let mut snap = Snapshot {
            view: "tree".into(),
            ..Snapshot::default()
        };
        snap.views.insert(
            "tree".into(),
            vec![SnapRow {
                display: "\u{1b}[32mrepo\u{1b}[0m [main]".into(),
                value: "/w/repo".into(),
                kind: "repo".into(),
            }],
        );
        snap.sessions.insert(
            "/w/repo".into(),
            vec![SnapRow {
                display: "main".into(),
                value: "abc".into(),
                kind: "session".into(),
            }],
        );
        snap
    }

    #[test]
    fn round_trip_through_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = path_in(dir.path());
        save_to(&path, &sample()).unwrap();
        assert_eq!(load_from(&path), Some(sample()));
        // No temp file is left behind.
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn missing_corrupt_and_mismatched_files_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = path_in(dir.path());
        assert_eq!(load_from(&path), None);

        std::fs::write(&path, "this is [not toml").unwrap();
        assert_eq!(load_from(&path), None);

        std::fs::write(&path, "version = 1\nview = 5\n").unwrap();
        assert_eq!(load_from(&path), None);

        let mut other = sample();
        other.version = VERSION + 1;
        save_to(&path, &other).unwrap();
        assert_eq!(load_from(&path), None);
    }

    #[test]
    fn seeded_rows_are_stale_and_record_skips_stale_levels() {
        let snap = sample();
        let mut level = Level::new(
            0,
            Source::View(ViewKind::Tree),
            "tree".into(),
            SortMode::Alpha,
        );
        assert!(snap.seed(&mut level));
        assert!(level.stale);
        assert_eq!(level.list.selected().unwrap().value, "/w/repo");
        assert!(matches!(
            level.list.selected().unwrap().kind,
            RowKind::Cached(_)
        ));

        let mut copy = Snapshot::default();
        copy.record(&level);
        assert!(copy.views.is_empty(), "stale rows must not be re-saved");

        level.stale = false;
        level.gen = 1;
        copy.record(&level);
        assert_eq!(copy.views["tree"][0].kind, "repo");

        let mut other = Level::new(
            1,
            Source::View(ViewKind::Repo),
            "repo".into(),
            SortMode::Alpha,
        );
        assert!(!snap.seed(&mut other));
    }
}
