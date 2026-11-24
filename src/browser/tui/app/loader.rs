//! Row loading and branch lookups on the worker pool.

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use crate::browser::tui::level::{self, Level, Loaded};
use crate::browser::tui::workers::Pool;
use crate::browser::BranchCache;
use crate::config::model::EzConfig;

/// Messages from worker threads.
pub enum Msg {
    Rows {
        level: u64,
        gen: u64,
        loaded: Loaded,
        /// A background refresh (branch data): keeps the previews cached.
        quiet: bool,
        /// Branches the rows were built without; look them up next.
        pending: Vec<PathBuf>,
        /// Early rows of a two-pass load; the full result (same `gen`) follows.
        partial: bool,
    },
    Failed {
        level: u64,
        gen: u64,
        error: String,
    },
    /// One branch lookup requested by `level` finished and is in the cache.
    Branch {
        level: u64,
    },
    /// A slow preview section finished (`index` is its segment).
    Preview {
        key: String,
        data_gen: u64,
        index: usize,
        text: String,
    },
}

/// What workers need to build rows, plus the way back to the app.
pub(super) struct Loader {
    pub config: Arc<EzConfig>,
    pub cache: Arc<BranchCache>,
    pub pool: Pool<Msg>,
    /// Direct line to the app, for partial results sent mid-job.
    pub tx: Sender<Msg>,
}

impl Loader {
    /// Build the level's rows on a worker; the status line shows "loading...".
    pub fn request(&self, level: &mut Level) {
        self.request_as(level, false);
    }

    /// Rebuild the rows without any visible loading state.
    pub fn request_quiet(&self, level: &mut Level) {
        self.request_as(level, true);
    }

    fn request_as(&self, level: &mut Level, quiet: bool) {
        level.gen += 1;
        level.busy = true;
        level.loading = !quiet;
        level.error = None;
        let (id, gen) = (level.id, level.gen);
        let source = level.source.clone();
        let sort = level.sort;
        let touch = level.take_touch();
        let config = Arc::clone(&self.config);
        let cache = Arc::clone(&self.cache);
        let tx = self.tx.clone();
        log::debug!("tui: load level {id} {source:?} (generation {gen}, quiet {quiet})");
        self.pool.submit_front(move || {
            // Quiet reloads already show rows; an early pass would only flicker.
            let partial = |loaded| {
                if quiet {
                    return;
                }
                let _ = tx.send(Msg::Rows {
                    level: id,
                    gen,
                    loaded,
                    quiet,
                    pending: Vec::new(),
                    partial: true,
                });
            };
            Some(
                match level::load(&source, sort, touch, &config, &cache, &partial) {
                    Ok(loaded) => Msg::Rows {
                        level: id,
                        gen,
                        loaded,
                        quiet,
                        pending: cache.take_pending(),
                        partial: false,
                    },
                    Err(e) => Msg::Failed {
                        level: id,
                        gen,
                        error: e.to_string(),
                    },
                },
            )
        });
    }

    /// Look up branches for `level` in the background; each finished lookup
    /// sends [`Msg::Branch`].
    pub fn resolve_branches(&self, level: u64, paths: Vec<PathBuf>) {
        log::debug!("tui: resolving {} branches for level {level}", paths.len());
        for path in paths {
            let cache = Arc::clone(&self.cache);
            self.pool.submit(move || {
                cache.resolve(&path);
                Some(Msg::Branch { level })
            });
        }
    }
}
