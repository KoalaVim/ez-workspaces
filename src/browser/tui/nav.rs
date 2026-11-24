//! Navigation stack (spec "Navigation stack"): the bottom entry is the
//! top-most level (a view); Esc pops one level and only exits at the bottom.

/// What Esc did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Back {
    /// A level was removed; the one below is shown again.
    Popped,
    /// Already at the top-most level: leave the browser.
    Exit,
}

/// A non-empty stack of levels.
#[derive(Debug)]
pub struct Nav<T> {
    levels: Vec<T>,
}

impl<T> Nav<T> {
    pub fn new(root: T) -> Self {
        Self { levels: vec![root] }
    }

    pub fn push(&mut self, level: T) {
        self.levels.push(level);
    }

    /// Esc: pop one level, never the root.
    pub fn back(&mut self) -> Back {
        if self.levels.len() > 1 {
            self.levels.pop();
            Back::Popped
        } else {
            Back::Exit
        }
    }

    #[cfg(test)]
    pub fn depth(&self) -> usize {
        self.levels.len()
    }

    pub fn is_root(&self) -> bool {
        self.levels.len() == 1
    }

    pub fn top(&self) -> &T {
        self.levels.last().expect("nav stack is never empty")
    }

    pub fn top_mut(&mut self) -> &mut T {
        self.levels.last_mut().expect("nav stack is never empty")
    }

    pub fn root(&self) -> &T {
        &self.levels[0]
    }

    /// Replace the root level, returning the old one. Levels above it stay.
    pub fn replace_root(&mut self, new_root: T) -> T {
        std::mem::replace(&mut self.levels[0], new_root)
    }

    /// Drop every level above the root.
    pub fn truncate_to_root(&mut self) {
        self.levels.truncate(1);
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.levels.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.levels.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_pops_one_level_at_a_time() {
        let mut nav = Nav::new("view");
        nav.push("sessions");
        nav.push("menu");
        assert_eq!(nav.depth(), 3);
        assert_eq!(nav.back(), Back::Popped);
        assert_eq!(*nav.top(), "sessions");
        assert_eq!(nav.back(), Back::Popped);
        assert_eq!(*nav.top(), "view");
        assert!(nav.is_root());
    }

    #[test]
    fn esc_at_root_exits_and_keeps_root() {
        let mut nav = Nav::new("view");
        assert_eq!(nav.back(), Back::Exit);
        assert_eq!(nav.back(), Back::Exit);
        assert_eq!(nav.depth(), 1);
        assert_eq!(*nav.top(), "view");
    }

    #[test]
    fn replace_root_keeps_levels_above() {
        let mut nav = Nav::new("a");
        nav.push("child");
        assert_eq!(nav.replace_root("b"), "a");
        assert_eq!(*nav.root(), "b");
        assert_eq!(*nav.top(), "child");
    }

    #[test]
    fn truncate_to_root_drops_children() {
        let mut nav = Nav::new("a");
        nav.push("x");
        nav.push("y");
        nav.truncate_to_root();
        assert!(nav.is_root());
        assert_eq!(*nav.top(), "a");
    }

    #[test]
    fn popped_level_state_is_gone_but_parent_state_survives() {
        let mut nav = Nav::new(vec![1, 2, 3]);
        nav.top_mut().push(4);
        nav.push(vec![9]);
        nav.back();
        assert_eq!(nav.top(), &vec![1, 2, 3, 4]);
    }
}
