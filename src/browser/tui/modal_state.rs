//! Key handling of a modal prompt as a pure state machine (no terminal), so
//! the Esc / Ctrl-P / Ctrl-C / typed-query semantics of `FzfSelector` can be
//! unit-tested. `modal.rs` draws it and feeds it keys.

use std::collections::BTreeSet;

use crossterm::event::{KeyCode, KeyModifiers};

use super::keys::Key;
use super::list::FilteredList;
use crate::browser::domain::{Row, RowKind};
use crate::browser::selector::SelectItem;
use crate::error::{EzError, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Fuzzy-filtered list; `multi` allows Tab marks.
    Pick { multi: bool },
    /// Free-text input.
    Text,
    /// Yes/no question.
    Confirm,
}

/// How a modal ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Done {
    /// Enter on a list item (index into the original items).
    Index(usize),
    /// Enter with nothing matching: the typed query.
    Query(String),
    /// Enter in a multi-select: marked items (or the highlighted one).
    Marked(Vec<usize>),
    /// Enter in a text modal.
    Text(String),
    /// Answer of a confirm modal.
    Answer(bool),
    /// An expect-key was pressed on an item.
    Action(String, usize),
    Back,
    Cancel,
}

pub enum Step {
    Pending,
    Done(Done),
}

pub struct Modal {
    pub title: String,
    /// Extra lines above the input (ANSI allowed): context, hints.
    pub header: Vec<String>,
    pub kind: Kind,
    pub list: FilteredList,
    pub text: String,
    pub confirm_default: bool,
    allow_back: bool,
    displays: Vec<String>,
    marked: BTreeSet<usize>,
    actions: Vec<(Key, String)>,
}

fn ctrl(c: char) -> Key {
    Key::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

impl Modal {
    fn base(title: &str, kind: Kind) -> Self {
        Self {
            title: title.to_string(),
            header: Vec::new(),
            kind,
            list: FilteredList::new(Vec::new()),
            text: String::new(),
            confirm_default: false,
            allow_back: false,
            displays: Vec::new(),
            marked: BTreeSet::new(),
            actions: Vec::new(),
        }
    }

    pub fn pick(title: &str, items: &[SelectItem], multi: bool) -> Self {
        let mut m = Self::base(title, Kind::Pick { multi });
        m.displays = items.iter().map(|i| i.display.clone()).collect();
        m.rebuild_rows();
        m
    }

    pub fn text(title: &str, default: Option<&str>) -> Self {
        let mut m = Self::base(title, Kind::Text);
        m.text = default.unwrap_or_default().to_string();
        m
    }

    pub fn confirm(question: &str, default: bool) -> Self {
        let mut m = Self::base(question, Kind::Confirm);
        m.confirm_default = default;
        m
    }

    /// Ctrl-P ends the modal with `Done::Back`.
    pub fn with_back(mut self, allow_back: bool) -> Self {
        self.allow_back = allow_back;
        self
    }

    pub fn with_header(mut self, lines: Vec<String>) -> Self {
        self.header = lines;
        self
    }

    /// Keys (fzf syntax) that end the modal with `Done::Action`. Unparseable keys are skipped.
    pub fn with_actions(mut self, keys: &[&str]) -> Self {
        for spec in keys {
            match Key::parse(spec) {
                Ok(key) => self.actions.push((key, spec.to_string())),
                Err(e) => log::warn!("modal: skipping key '{spec}': {e}"),
            }
        }
        self
    }

    /// The text being edited (list filter or free text).
    pub fn input(&self) -> &str {
        match self.kind {
            Kind::Text => &self.text,
            _ => self.list.query(),
        }
    }

    fn rebuild_rows(&mut self) {
        let multi = matches!(self.kind, Kind::Pick { multi: true });
        let rows = self
            .displays
            .iter()
            .enumerate()
            .map(|(i, d)| Row {
                display: if multi {
                    format!("[{}] {d}", if self.marked.contains(&i) { 'x' } else { ' ' })
                } else {
                    d.clone()
                },
                value: i.to_string(),
                kind: RowKind::Header,
            })
            .collect();
        self.list.set_rows(rows);
    }

    fn selected_index(&self) -> Option<usize> {
        self.list.selected().and_then(|r| r.value.parse().ok())
    }

    pub fn on_key(&mut self, key: Key) -> Result<Step> {
        if key == ctrl('c') {
            return Err(EzError::Interrupted);
        }
        let done = |d| Ok(Step::Done(d));
        match self.kind {
            Kind::Confirm => match (key.code, key.mods) {
                (KeyCode::Char('y' | 'Y'), m) if m.is_empty() => done(Done::Answer(true)),
                (KeyCode::Char('n' | 'N'), m) if m.is_empty() => done(Done::Answer(false)),
                (KeyCode::Enter, _) => done(Done::Answer(self.confirm_default)),
                (KeyCode::Esc, _) => done(Done::Answer(false)),
                _ => Ok(Step::Pending),
            },
            Kind::Text => {
                if key.code == KeyCode::Esc {
                    return done(Done::Cancel);
                }
                if self.allow_back && key == ctrl('p') {
                    return done(Done::Back);
                }
                if key.code == KeyCode::Enter {
                    return done(Done::Text(self.text.clone()));
                }
                edit(&mut self.text, key);
                Ok(Step::Pending)
            }
            Kind::Pick { multi } => self.on_pick_key(key, multi),
        }
    }

    fn on_pick_key(&mut self, key: Key, multi: bool) -> Result<Step> {
        let done = |d| Ok(Step::Done(d));
        if key.code == KeyCode::Esc {
            return done(Done::Cancel);
        }
        if self.allow_back && key == ctrl('p') {
            return done(Done::Back);
        }
        if let Some((_, spec)) = self.actions.iter().find(|(k, _)| *k == key) {
            return Ok(match self.selected_index() {
                Some(idx) => Step::Done(Done::Action(spec.clone(), idx)),
                None => Step::Pending,
            });
        }
        match (key.code, key.mods) {
            (KeyCode::Enter, _) => {
                if multi {
                    let mut picked: Vec<usize> = self.marked.iter().copied().collect();
                    if picked.is_empty() {
                        picked.extend(self.selected_index());
                    }
                    return done(Done::Marked(picked));
                }
                match self.selected_index() {
                    Some(idx) => done(Done::Index(idx)),
                    None => done(Done::Query(self.list.query().to_string())),
                }
            }
            (KeyCode::Tab, _) if multi => self.toggle(1),
            (KeyCode::BackTab, _) if multi => self.toggle(-1),
            (KeyCode::Up, _) | (KeyCode::Char('p' | 'k'), KeyModifiers::CONTROL) => {
                self.list.move_cursor(-1);
                Ok(Step::Pending)
            }
            (KeyCode::Down, _) | (KeyCode::Char('n' | 'j'), KeyModifiers::CONTROL) => {
                self.list.move_cursor(1);
                Ok(Step::Pending)
            }
            (KeyCode::PageUp, _) => {
                self.list.move_cursor(-10);
                Ok(Step::Pending)
            }
            (KeyCode::PageDown, _) => {
                self.list.move_cursor(10);
                Ok(Step::Pending)
            }
            _ => {
                let mut query = self.list.query().to_string();
                if edit(&mut query, key) {
                    self.list.set_query(&query);
                }
                Ok(Step::Pending)
            }
        }
    }

    fn toggle(&mut self, step: isize) -> Result<Step> {
        if let Some(idx) = self.selected_index() {
            if !self.marked.remove(&idx) {
                self.marked.insert(idx);
            }
            self.rebuild_rows();
            self.list.move_cursor(step);
        }
        Ok(Step::Pending)
    }
}

/// Apply a text-editing key to `text`; returns whether it changed.
fn edit(text: &mut String, key: Key) -> bool {
    match (key.code, key.mods) {
        (KeyCode::Backspace, _) => text.pop().is_some(),
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
            let changed = !text.is_empty();
            text.clear();
            changed
        }
        (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
            let trimmed = text.trim_end().len();
            let start = text[..trimmed]
                .rfind(char::is_whitespace)
                .map(|i| i + 1)
                .unwrap_or(0);
            let changed = start != text.len();
            text.truncate(start);
            changed
        }
        (KeyCode::Char(c), m) if m.is_empty() => {
            text.push(c);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(names: &[&str]) -> Vec<SelectItem> {
        names
            .iter()
            .map(|n| SelectItem {
                display: n.to_string(),
                value: format!("v-{n}"),
            })
            .collect()
    }

    fn press(m: &mut Modal, keys: &[&str]) -> Done {
        for k in keys {
            let key = if k.chars().count() == 1 {
                Key::new(KeyCode::Char(k.chars().next().unwrap()), KeyModifiers::NONE)
            } else {
                Key::parse(k).unwrap()
            };
            if let Step::Done(d) = m.on_key(key).unwrap() {
                return d;
            }
        }
        panic!("modal did not finish");
    }

    fn type_str(m: &mut Modal, s: &str) {
        for c in s.chars() {
            let step = m
                .on_key(Key::new(KeyCode::Char(c), KeyModifiers::NONE))
                .unwrap();
            assert!(matches!(step, Step::Pending));
        }
    }

    #[test]
    fn enter_picks_highlighted_item() {
        let mut m = Modal::pick("p", &items(&["alpha", "beta"]), false);
        assert_eq!(press(&mut m, &["down", "enter"]), Done::Index(1));
    }

    #[test]
    fn typing_filters_and_picks_the_match() {
        let mut m = Modal::pick("p", &items(&["alpha", "beta"]), false);
        type_str(&mut m, "bet");
        assert_eq!(press(&mut m, &["enter"]), Done::Index(1));
    }

    #[test]
    fn typed_query_without_match_is_returned() {
        let mut m = Modal::pick("p", &items(&["alpha", "beta"]), false).with_back(true);
        type_str(&mut m, "zzz");
        assert_eq!(press(&mut m, &["enter"]), Done::Query("zzz".into()));
    }

    #[test]
    fn esc_cancels_and_ctrl_p_goes_back_only_when_allowed() {
        let mut m = Modal::pick("p", &items(&["a"]), false).with_back(true);
        assert_eq!(press(&mut m, &["ctrl-p"]), Done::Back);
        let mut m = Modal::pick("p", &items(&["a"]), false);
        assert!(matches!(m.on_key(ctrl('p')).unwrap(), Step::Pending));
        assert_eq!(press(&mut m, &["esc"]), Done::Cancel);
        let mut t = Modal::text("p", None).with_back(true);
        assert_eq!(press(&mut t, &["ctrl-p"]), Done::Back);
    }

    #[test]
    fn ctrl_c_is_interrupted_in_every_kind() {
        for mut m in [
            Modal::pick("p", &items(&["a"]), false),
            Modal::text("p", None),
            Modal::confirm("sure?", false),
        ] {
            assert!(matches!(m.on_key(ctrl('c')), Err(EzError::Interrupted)));
        }
    }

    #[test]
    fn text_modal_edits_and_returns_text() {
        let mut m = Modal::text("name", Some("ab"));
        type_str(&mut m, "cd");
        assert_eq!(m.input(), "abcd");
        assert_eq!(
            press(&mut m, &["bspace", "enter"]),
            Done::Text("abc".into())
        );
        let mut m = Modal::text("name", Some("foo bar"));
        m.on_key(ctrl('w')).unwrap();
        assert_eq!(m.input(), "foo ");
        m.on_key(ctrl('u')).unwrap();
        assert_eq!(press(&mut m, &["enter"]), Done::Text(String::new()));
        let mut m = Modal::text("name", Some("x"));
        assert_eq!(press(&mut m, &["esc"]), Done::Cancel);
    }

    #[test]
    fn confirm_answers() {
        assert_eq!(
            press(&mut Modal::confirm("q", false), &["y"]),
            Done::Answer(true)
        );
        assert_eq!(
            press(&mut Modal::confirm("q", true), &["n"]),
            Done::Answer(false)
        );
        assert_eq!(
            press(&mut Modal::confirm("q", true), &["enter"]),
            Done::Answer(true)
        );
        assert_eq!(
            press(&mut Modal::confirm("q", true), &["esc"]),
            Done::Answer(false)
        );
        let mut m = Modal::confirm("q", false);
        assert!(matches!(
            m.on_key(Key::new(KeyCode::Char('x'), KeyModifiers::NONE))
                .unwrap(),
            Step::Pending
        ));
    }

    #[test]
    fn multi_select_marks_with_tab_and_falls_back_to_highlight() {
        let mut m = Modal::pick("p", &items(&["a", "b", "c"]), true);
        assert_eq!(
            press(&mut m, &["tab", "tab", "enter"]),
            Done::Marked(vec![0, 1])
        );
        let mut m = Modal::pick("p", &items(&["a", "b"]), true);
        assert_eq!(press(&mut m, &["down", "enter"]), Done::Marked(vec![1]));
    }

    #[test]
    fn expect_keys_report_the_highlighted_item() {
        let mut m = Modal::pick("p", &items(&["a", "b"]), false).with_actions(&["alt-d", "bogus-"]);
        assert_eq!(
            press(&mut m, &["down", "alt-d"]),
            Done::Action("alt-d".into(), 1)
        );
        // No highlighted item: the key is ignored.
        let mut m = Modal::pick("p", &[], false).with_actions(&["alt-d"]);
        let key = Key::parse("alt-d").unwrap();
        assert!(matches!(m.on_key(key).unwrap(), Step::Pending));
    }
}
