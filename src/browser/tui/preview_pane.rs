//! Preview pane: renders the domain preview (ANSI text) with `ansi-to-tui`,
//! cached per (request, data generation).
//!
//! The fast part of a preview is rendered on the main thread at once. Slow
//! sections (git status/log, bat notes, plugin preview commands) are queued
//! on the worker pool and show a loading placeholder until their result
//! arrives as `Msg::Preview`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use ansi_to_tui::IntoText;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;

use crate::browser::tui::theme;
use ratatui::Frame;

use super::app::Msg;
use super::level::PreviewReq;
use super::workers::Pool;
use crate::browser::domain::preview::{preview_segments, Segment, Slow};

const PLACEHOLDER: &str = "\x1b[2m  loading…\x1b[0m\n";

struct Entry {
    segments: Vec<Segment>,
    /// Finished slow sections by segment index.
    done: HashMap<usize, String>,
    text: Text<'static>,
}

impl Entry {
    fn new(segments: Vec<Segment>) -> Self {
        let mut entry = Self {
            segments,
            done: HashMap::new(),
            text: Text::default(),
        };
        entry.rebuild();
        entry
    }

    fn rebuild(&mut self) {
        let mut raw = String::new();
        for (i, seg) in self.segments.iter().enumerate() {
            match seg {
                Segment::Text(t) => raw.push_str(t),
                Segment::Slow(_) => raw.push_str(self.done.get(&i).map_or(PLACEHOLDER, |s| s)),
            }
        }
        self.text = theme::section_headers(to_text(&raw));
    }
}

fn to_text(raw: &str) -> Text<'static> {
    raw.into_text()
        .unwrap_or_else(|_| Text::raw(console::strip_ansi_codes(raw).to_string()))
}

pub struct PreviewPane {
    cache: HashMap<(String, u64), Entry>,
    pool: Pool<Msg>,
    /// Data generation the app is at; queued jobs of older ones are skipped.
    current_gen: Arc<AtomicU64>,
}

impl PreviewPane {
    pub fn new(pool: Pool<Msg>) -> Self {
        Self {
            cache: HashMap::new(),
            pool,
            current_gen: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Entry for `req`, rendered (fast part) and queued (slow parts) on a miss.
    fn entry(&mut self, req: &PreviewReq, data_gen: u64) -> &Entry {
        let key = (req.key(), data_gen);
        self.current_gen.store(data_gen, Ordering::Relaxed);
        if !self.cache.contains_key(&key) {
            // Entries from older data generations can never be hit again.
            self.cache.retain(|(_, g), _| *g == data_gen);
            log::debug!("preview: render {:?} (generation {data_gen})", req.path);
            let segments = match &req.cmd {
                Some(cmd) => Ok(vec![Segment::Slow(Slow::Shell(cmd.clone()))]),
                None => preview_segments(&req.path, req.session_actions, req.session_id.as_deref()),
            }
            .unwrap_or_else(|e| vec![Segment::Text(format!("preview failed: {e}\n"))]);
            for (index, seg) in segments.iter().enumerate() {
                if let Segment::Slow(slow) = seg {
                    self.queue(&key.0, data_gen, index, slow.clone());
                }
            }
            self.cache.insert(key.clone(), Entry::new(segments));
        }
        &self.cache[&key]
    }

    fn queue(&self, key: &str, data_gen: u64, index: usize, slow: Slow) {
        let (key, current) = (key.to_string(), Arc::clone(&self.current_gen));
        self.pool.submit(move || {
            if current.load(Ordering::Relaxed) != data_gen {
                log::debug!("preview: skipped outdated section for {key}");
                return None;
            }
            let started = std::time::Instant::now();
            let text = slow.render();
            log::debug!(
                "preview: section {index} of {key} took {:?}",
                started.elapsed()
            );
            Some(Msg::Preview {
                key,
                data_gen,
                index,
                text,
            })
        });
    }

    /// A worker finished a slow section.
    pub fn fill(&mut self, key: String, data_gen: u64, index: usize, text: String) {
        if let Some(entry) = self.cache.get_mut(&(key, data_gen)) {
            entry.done.insert(index, text);
            entry.rebuild();
        }
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        req: Option<&PreviewReq>,
        data_gen: u64,
    ) {
        let block = theme::panel("Preview", false);
        let text = match req {
            Some(req) => self.entry(req, data_gen).text.clone(),
            None => Text::default(),
        };
        frame.render_widget(Paragraph::new(text).block(block), area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    fn plain(text: &Text<'_>) -> String {
        text.lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn slow_section_shows_a_placeholder_until_its_result_arrives() {
        let (tx, rx) = mpsc::channel();
        let mut pane = PreviewPane::new(Pool::new(tx));
        let req = PreviewReq {
            path: Default::default(),
            session_actions: false,
            session_id: None,
            cmd: Some("echo from-plugin".into()),
        };
        let key = req.key();
        assert!(plain(&pane.entry(&req, 1).text).contains("loading…"));
        let Msg::Preview {
            key: k,
            data_gen,
            index,
            text,
        } = rx.recv_timeout(Duration::from_secs(5)).unwrap()
        else {
            panic!("expected a preview result");
        };
        assert_eq!((k.as_str(), data_gen, index), (key.as_str(), 1, 0));
        pane.fill(k, data_gen, index, text);
        let shown = plain(&pane.entry(&req, 1).text);
        assert!(
            shown.contains("from-plugin") && !shown.contains("loading"),
            "{shown}"
        );
    }
}
