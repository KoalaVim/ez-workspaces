//! Where the app reads and writes the snapshot (the ez data directory).

use std::path::PathBuf;

use super::App;
use crate::browser::tui::level;
use crate::browser::tui::snapshot::{self, Snapshot};
use crate::config::model::EzConfig;
use crate::paths;

/// The snapshot to start from and the file to save to; both `None` when
/// `[native] snapshot` is off or there is no data directory.
pub(super) fn load(config: &EzConfig) -> (Option<Snapshot>, Option<PathBuf>) {
    if !config.native.snapshot {
        log::debug!("snapshot: disabled by config");
        return (None, None);
    }
    let Ok(dir) = paths::data_dir() else {
        return (None, None);
    };
    let path = snapshot::path_in(&dir);
    let snap = snapshot::load_from(&path).unwrap_or_default();
    level::seed_attached(&snap.attached);
    (Some(snap), Some(path))
}

impl App {
    /// Write the rows of every loaded level (merged over the older snapshot).
    pub(super) fn save_snapshot(&mut self) {
        let (Some(snap), Some(path)) = (self.snapshot.as_mut(), self.snapshot_path.as_ref()) else {
            return;
        };
        for level in self.nav.iter().chain(self.saved.values()) {
            snap.record(level);
        }
        snap.view = self.nav.root().title.clone();
        snap.attached.extend(level::attached_memo());
        if let Err(e) = snapshot::save_to(path, snap) {
            log::debug!("snapshot: save failed: {e}");
        }
    }
}
