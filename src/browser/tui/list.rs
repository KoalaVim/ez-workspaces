//! Fuzzy-filtered list (design D5): matching runs on ANSI-stripped display
//! text with `nucleo-matcher`; rows keep their ANSI display for rendering.

use ansi_to_tui::IntoText;
use console::strip_ansi_codes;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

use crate::browser::domain::Row;

/// One row that passed the filter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    /// Index into the full row list.
    pub index: usize,
    pub score: u32,
    /// Matched character positions in the ANSI-stripped text (sorted, unique).
    pub indices: Vec<u32>,
}

pub struct FilteredList {
    rows: Vec<Row>,
    plain: Vec<String>,
    query: String,
    hits: Vec<Hit>,
    cursor: usize,
    matcher: Matcher,
    state: ListState,
}

impl FilteredList {
    pub fn new(rows: Vec<Row>) -> Self {
        let mut list = Self {
            rows: Vec::new(),
            plain: Vec::new(),
            query: String::new(),
            hits: Vec::new(),
            cursor: 0,
            matcher: Matcher::new(Config::DEFAULT),
            state: ListState::default(),
        };
        list.set_rows(rows);
        list
    }

    /// Replace the rows, keeping the cursor on the same `Row::value` when it still exists.
    pub fn set_rows(&mut self, rows: Vec<Row>) {
        let keep = self.selected().map(|r| r.value.clone());
        self.plain = rows
            .iter()
            .map(|r| strip_ansi_codes(&r.display).to_string())
            .collect();
        self.rows = rows;
        self.refilter();
        if let Some(value) = keep {
            if let Some(pos) = self
                .hits
                .iter()
                .position(|h| self.rows[h.index].value == value)
            {
                self.cursor = pos;
            }
        }
        self.sync_state();
    }

    /// All rows, unfiltered.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.refilter();
        self.sync_state();
    }

    pub fn push_char(&mut self, c: char) {
        self.query.push(c);
        self.refilter();
        self.sync_state();
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
        self.refilter();
        self.sync_state();
    }

    #[cfg(test)]
    pub fn hits(&self) -> &[Hit] {
        &self.hits
    }

    pub fn len(&self) -> usize {
        self.hits.len()
    }

    pub fn total(&self) -> usize {
        self.rows.len()
    }

    #[cfg(test)]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Move the cursor by `delta`, clamped to the filtered list.
    pub fn move_cursor(&mut self, delta: isize) {
        if self.hits.is_empty() {
            return;
        }
        let max = self.hits.len() as isize - 1;
        self.cursor = (self.cursor as isize + delta).clamp(0, max) as usize;
        self.sync_state();
    }

    pub fn selected(&self) -> Option<&Row> {
        self.hits.get(self.cursor).map(|h| &self.rows[h.index])
    }

    fn refilter(&mut self) {
        let previous = self.hits.get(self.cursor).map(|h| h.index);
        self.hits = if self.query.is_empty() {
            (0..self.rows.len())
                .map(|index| Hit {
                    index,
                    score: 0,
                    indices: Vec::new(),
                })
                .collect()
        } else {
            let pattern = Pattern::parse(&self.query, CaseMatching::Smart, Normalization::Smart);
            let mut buf = Vec::new();
            let mut hits: Vec<Hit> = self
                .plain
                .iter()
                .enumerate()
                .filter_map(|(index, text)| {
                    let mut indices = Vec::new();
                    let hay = Utf32Str::new(text, &mut buf);
                    let score = pattern.indices(hay, &mut self.matcher, &mut indices)?;
                    indices.sort_unstable();
                    indices.dedup();
                    Some(Hit {
                        index,
                        score,
                        indices,
                    })
                })
                .collect();
            // Best score first; ties keep the original row order.
            hits.sort_by(|a, b| b.score.cmp(&a.score).then(a.index.cmp(&b.index)));
            hits
        };
        // Cursor: stay on the same row if it survived the filter, else go to the top.
        self.cursor = previous
            .and_then(|p| self.hits.iter().position(|h| h.index == p))
            .unwrap_or(0);
        log::debug!(
            "list: query '{}' -> {}/{} rows",
            self.query,
            self.hits.len(),
            self.rows.len()
        );
    }

    fn sync_state(&mut self) {
        self.state.select(if self.hits.is_empty() {
            None
        } else {
            Some(self.cursor)
        });
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .hits
            .iter()
            .map(|h| ListItem::new(display_line(&self.rows[h.index].display, &h.indices)))
            .collect();
        let list = List::new(items)
            .highlight_symbol("> ")
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(list, area, &mut self.state);
    }
}

/// Convert an ANSI display string to a `Line`, highlighting matched characters.
pub fn display_line(display: &str, matched: &[u32]) -> Line<'static> {
    let line = match display.into_text() {
        Ok(text) => text.lines.into_iter().next().unwrap_or_default(),
        Err(_) => Line::from(strip_ansi_codes(display).to_string()),
    };
    highlight(line, matched)
}

/// Patch matched character positions (counted across the line's spans) with a highlight style.
pub fn highlight(line: Line<'static>, matched: &[u32]) -> Line<'static> {
    if matched.is_empty() {
        return line;
    }
    let hl = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let (style, alignment) = (line.style, line.alignment);
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut pos: u32 = 0;
    for span in line.spans {
        let mut run = String::new();
        let mut run_hl = false;
        for ch in span.content.chars() {
            let is_hl = matched.binary_search(&pos).is_ok();
            if is_hl != run_hl && !run.is_empty() {
                spans.push(styled(std::mem::take(&mut run), span.style, run_hl, hl));
            }
            run_hl = is_hl;
            run.push(ch);
            pos += 1;
        }
        if !run.is_empty() {
            spans.push(styled(run, span.style, run_hl, hl));
        }
    }
    Line {
        style,
        alignment,
        spans,
    }
}

fn styled(text: String, base: Style, highlighted: bool, hl: Style) -> Span<'static> {
    Span::styled(text, if highlighted { base.patch(hl) } else { base })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::domain::RowKind;

    fn row(display: &str, value: &str) -> Row {
        Row {
            display: display.into(),
            value: value.into(),
            kind: RowKind::Header,
        }
    }

    fn values(list: &FilteredList) -> Vec<String> {
        list.hits()
            .iter()
            .map(|h| list.rows[h.index].value.clone())
            .collect()
    }

    fn sample() -> FilteredList {
        FilteredList::new(vec![
            row("alpha-service", "a"),
            row("beta", "b"),
            row("service-alpha", "c"),
            row("gamma", "d"),
        ])
    }

    #[test]
    fn empty_query_keeps_order() {
        let list = sample();
        assert_eq!(values(&list), ["a", "b", "c", "d"]);
        assert_eq!(list.selected().unwrap().value, "a");
    }

    #[test]
    fn filters_and_ranks_by_score() {
        let mut list = FilteredList::new(vec![
            row("some_long_prefix_then_ser", "weak"),
            row("serv", "exact"),
            row("unrelated", "none"),
        ]);
        list.set_query("serv");
        let v = values(&list);
        assert_eq!(v.first().map(String::as_str), Some("exact"));
        assert!(!v.contains(&"none".to_string()));
    }

    #[test]
    fn ansi_codes_are_ignored_when_matching() {
        let mut list = FilteredList::new(vec![
            row("\x1b[1;34mfoo\x1b[0m\x1b[2m(bar)\x1b[0m", "x"),
            row("baz", "y"),
        ]);
        // "34m" and "1;" only exist inside escape sequences.
        list.set_query("34m");
        assert_eq!(list.len(), 0);
        list.set_query("foobar");
        assert_eq!(values(&list), ["x"]);
        // indices refer to the stripped text "foo(bar)"
        assert_eq!(list.hits()[0].indices[..3], [0, 1, 2]);
    }

    #[test]
    fn cursor_resets_or_follows_on_filter_change() {
        let mut list = sample();
        list.move_cursor(2);
        assert_eq!(list.selected().unwrap().value, "c");
        list.set_query("alpha");
        assert!(list.cursor() < list.len());
        assert!(list.selected().is_some());
        // cursor follows its row when it survives
        list.set_query("");
        list.move_cursor(-99);
        list.move_cursor(2);
        list.set_query("a");
        assert_eq!(list.selected().unwrap().value, "c");
        // filtered to nothing
        list.set_query("zzzzz");
        assert!(list.selected().is_none());
        list.move_cursor(1);
        assert_eq!(list.cursor(), 0);
    }

    #[test]
    fn cursor_clamps() {
        let mut list = sample();
        list.move_cursor(-5);
        assert_eq!(list.cursor(), 0);
        list.move_cursor(99);
        assert_eq!(list.cursor(), 3);
    }

    #[test]
    fn set_rows_keeps_cursor_on_same_value() {
        let mut list = sample();
        list.move_cursor(2);
        list.set_rows(vec![row("new", "n"), row("service-alpha", "c")]);
        assert_eq!(list.selected().unwrap().value, "c");
    }

    #[test]
    fn query_edit_helpers() {
        let mut list = sample();
        list.push_char('g');
        list.push_char('a');
        assert_eq!(list.query(), "ga");
        assert_eq!(values(&list)[0], "d");
        list.pop_char();
        list.pop_char();
        assert_eq!(list.total(), list.len());
    }

    #[test]
    fn highlight_marks_matched_chars_across_spans() {
        let line = display_line("\x1b[31mab\x1b[0mcd", &[1, 2]);
        let hl: Vec<(String, bool)> = line
            .spans
            .iter()
            .map(|s| (s.content.to_string(), s.style.fg == Some(Color::Yellow)))
            .collect();
        assert_eq!(
            hl,
            vec![
                ("a".to_string(), false),
                ("b".to_string(), true),
                ("c".to_string(), true),
                ("d".to_string(), false),
            ]
        );
    }
}
