use crate::error::{EzError, Result};

use super::model::{Session, SessionId, SessionTree};

/// A node in the rendered tree, carrying layout metadata for tree-glyph rendering.
pub struct TreeNode<'a> {
    pub depth: usize,
    pub session: &'a Session,
    pub is_last_sibling: bool,
    /// For each ancestor depth level, whether a `│` continuation line is needed.
    pub ancestor_has_next: Vec<bool>,
}

/// Render box-drawing connectors for a tree node.
///
/// Produces a prefix like `│   ├── ` or `    └── ` depending on the node's
/// position in the tree. Root nodes (depth 0) get no prefix.
pub fn format_session_tree_line(node: &TreeNode<'_>) -> String {
    if node.depth == 0 {
        return String::new();
    }
    let mut prefix = String::new();
    for &has_next in &node.ancestor_has_next {
        if has_next {
            prefix.push_str("│   ");
        } else {
            prefix.push_str("    ");
        }
    }
    if node.is_last_sibling {
        prefix.push_str("└── ");
    } else {
        prefix.push_str("├── ");
    }
    prefix
}

impl SessionTree {
    /// Get root-level sessions (no parent).
    pub fn roots(&self) -> Vec<&Session> {
        self.sessions
            .iter()
            .filter(|s| s.parent_id.is_none())
            .collect()
    }

    /// Get direct children of a session.
    pub fn children(&self, parent_id: &SessionId) -> Vec<&Session> {
        self.sessions
            .iter()
            .filter(|s| s.parent_id.as_ref() == Some(parent_id))
            .collect()
    }

    /// Get the full ancestry chain (bottom-up, excluding self).
    #[allow(dead_code)]
    pub fn ancestors(&self, session_id: &SessionId) -> Vec<&Session> {
        let mut result = Vec::new();
        let mut current_id = session_id.clone();

        while let Some(session) = self.find_by_id(&current_id) {
            let Some(pid) = session.parent_id.clone() else {
                break;
            };
            if let Some(parent) = self.find_by_id(&pid) {
                result.push(parent);
                current_id = pid;
            } else {
                break;
            }
        }
        result
    }

    /// Get all descendants recursively (for cascade delete).
    pub fn descendants(&self, session_id: &SessionId) -> Vec<&Session> {
        let mut result = Vec::new();
        let mut stack = vec![session_id.clone()];

        while let Some(id) = stack.pop() {
            for child in self.children(&id) {
                result.push(child);
                stack.push(child.id.clone());
            }
        }
        result
    }

    /// Render as a depth-first tree with layout metadata for tree-glyph rendering.
    pub fn render_tree(&self) -> Vec<TreeNode<'_>> {
        let mut result = Vec::new();
        let roots = self.roots();
        let num_roots = roots.len();
        for (i, root) in roots.iter().enumerate() {
            let is_last = i == num_roots - 1;
            self.render_subtree(root, 0, is_last, &[], &mut result);
        }
        result
    }

    /// Render the tree with root sessions sorted by most-recently-accessed first.
    /// Children within each root subtree preserve their original order.
    /// Sessions with no `last_accessed` sort to the end.
    pub fn render_tree_lru(&self) -> Vec<TreeNode<'_>> {
        let mut result = Vec::new();
        let mut roots = self.roots();
        roots.sort_by(|a, b| {
            let a_ts = self.subtree_max_accessed(&a.id);
            let b_ts = self.subtree_max_accessed(&b.id);
            match (&b_ts, &a_ts) {
                (Some(b_v), Some(a_v)) => b_v.cmp(a_v),
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        let num_roots = roots.len();
        for (i, root) in roots.iter().enumerate() {
            let is_last = i == num_roots - 1;
            self.render_subtree(root, 0, is_last, &[], &mut result);
        }
        result
    }

    /// Find the most recent `last_accessed` timestamp across a session and all
    /// its descendants. Returns `None` if none have been accessed.
    fn subtree_max_accessed(&self, session_id: &SessionId) -> Option<String> {
        let mut max: Option<String> = None;
        if let Some(session) = self.find_by_id(session_id) {
            if let Some(ref ts) = session.last_accessed {
                max = Some(ts.clone());
            }
        }
        for desc in self.descendants(session_id) {
            if let Some(ref ts) = desc.last_accessed {
                if max.as_ref().is_none_or(|m| ts > m) {
                    max = Some(ts.clone());
                }
            }
        }
        max
    }

    fn render_subtree<'a>(
        &'a self,
        session: &'a Session,
        depth: usize,
        is_last_sibling: bool,
        ancestor_has_next: &[bool],
        result: &mut Vec<TreeNode<'a>>,
    ) {
        result.push(TreeNode {
            depth,
            session,
            is_last_sibling,
            ancestor_has_next: ancestor_has_next.to_vec(),
        });
        let children = self.children(&session.id);
        let num_children = children.len();
        for (i, child) in children.iter().enumerate() {
            let child_is_last = i == num_children - 1;
            let mut child_ancestor = ancestor_has_next.to_vec();
            if depth > 0 || ancestor_has_next.is_empty() {
                // Only push continuation for nodes that have a connector prefix
                if depth > 0 {
                    child_ancestor.push(!is_last_sibling);
                }
            }
            self.render_subtree(child, depth + 1, child_is_last, &child_ancestor, result);
        }
    }

    /// Find the default session (the one with `is_default == true`).
    pub fn find_default(&self) -> Option<&Session> {
        self.sessions.iter().find(|s| s.is_default)
    }

    /// Find a session by name.
    pub fn find_by_name(&self, name: &str) -> Option<&Session> {
        self.sessions.iter().find(|s| s.name == name)
    }

    /// Find a session by ID.
    pub fn find_by_id(&self, id: &SessionId) -> Option<&Session> {
        self.sessions.iter().find(|s| s.id == *id)
    }

    /// Add a session. Errors if name already exists.
    pub fn add(&mut self, session: Session) -> Result<()> {
        if self.find_by_name(&session.name).is_some() {
            return Err(EzError::SessionAlreadyExists(session.name));
        }
        if let Some(pid) = &session.parent_id {
            if self.find_by_id(pid).is_none() {
                return Err(EzError::SessionNotFound(format!(
                    "parent session '{pid}' not found"
                )));
            }
        }
        self.sessions.push(session);
        Ok(())
    }

    /// Remove a session by ID. Returns the removed session.
    pub fn remove(&mut self, session_id: &SessionId) -> Result<Session> {
        let pos = self
            .sessions
            .iter()
            .position(|s| s.id == *session_id)
            .ok_or_else(|| EzError::SessionNotFound(session_id.clone()))?;
        Ok(self.sessions.remove(pos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_session(id: &str, name: &str, parent: Option<&str>) -> Session {
        Session {
            id: id.to_string(),
            name: name.to_string(),
            parent_id: parent.map(|s| s.to_string()),
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

    #[test]
    fn test_roots() {
        let tree = SessionTree {
            sessions: vec![
                make_session("1", "main", None),
                make_session("2", "feature", None),
                make_session("3", "sub", Some("2")),
            ],
        };
        let roots = tree.roots();
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn test_children() {
        let tree = SessionTree {
            sessions: vec![
                make_session("1", "main", None),
                make_session("2", "feature", None),
                make_session("3", "sub-a", Some("2")),
                make_session("4", "sub-b", Some("2")),
            ],
        };
        let children = tree.children(&"2".to_string());
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_descendants() {
        let tree = SessionTree {
            sessions: vec![
                make_session("1", "root", None),
                make_session("2", "child", Some("1")),
                make_session("3", "grandchild", Some("2")),
            ],
        };
        let desc = tree.descendants(&"1".to_string());
        assert_eq!(desc.len(), 2);
    }

    #[test]
    fn test_render_tree() {
        let tree = SessionTree {
            sessions: vec![
                make_session("1", "main", None),
                make_session("2", "feature", None),
                make_session("3", "sub", Some("2")),
            ],
        };
        let rendered = tree.render_tree();
        assert_eq!(rendered.len(), 3);
        assert_eq!(rendered[0].depth, 0); // main at depth 0
        assert_eq!(rendered[1].depth, 0); // feature at depth 0
        assert_eq!(rendered[2].depth, 1); // sub at depth 1
    }

    #[test]
    fn test_add_duplicate_name() {
        let mut tree = SessionTree::default();
        tree.add(make_session("1", "main", None)).unwrap();
        let result = tree.add(make_session("2", "main", None));
        assert!(result.is_err());
    }

    #[test]
    fn test_add_invalid_parent() {
        let mut tree = SessionTree::default();
        let result = tree.add(make_session("1", "child", Some("nonexistent")));
        assert!(result.is_err());
    }

    #[test]
    fn test_remove() {
        let mut tree = SessionTree::default();
        tree.add(make_session("1", "main", None)).unwrap();
        let removed = tree.remove(&"1".to_string()).unwrap();
        assert_eq!(removed.name, "main");
        assert!(tree.sessions.is_empty());
    }

    #[test]
    fn test_tree_glyphs_single_child() {
        let tree = SessionTree {
            sessions: vec![
                make_session("1", "root", None),
                make_session("2", "child", Some("1")),
            ],
        };
        let rendered = tree.render_tree();
        assert_eq!(rendered.len(), 2);

        assert_eq!(format_session_tree_line(&rendered[0]), "");
        assert!(rendered[0].is_last_sibling);

        assert_eq!(format_session_tree_line(&rendered[1]), "└── ");
        assert!(rendered[1].is_last_sibling);
    }

    #[test]
    fn test_tree_glyphs_multiple_siblings() {
        let tree = SessionTree {
            sessions: vec![
                make_session("1", "root", None),
                make_session("2", "child-a", Some("1")),
                make_session("3", "child-b", Some("1")),
                make_session("4", "child-c", Some("1")),
            ],
        };
        let rendered = tree.render_tree();
        assert_eq!(rendered.len(), 4);

        assert_eq!(format_session_tree_line(&rendered[0]), "");
        assert_eq!(format_session_tree_line(&rendered[1]), "├── ");
        assert!(!rendered[1].is_last_sibling);
        assert_eq!(format_session_tree_line(&rendered[2]), "├── ");
        assert!(!rendered[2].is_last_sibling);
        assert_eq!(format_session_tree_line(&rendered[3]), "└── ");
        assert!(rendered[3].is_last_sibling);
    }

    #[test]
    fn test_tree_glyphs_deep_nesting() {
        // root
        // ├── a
        // │   └── b
        // │       └── c
        // └── d
        let tree = SessionTree {
            sessions: vec![
                make_session("1", "root", None),
                make_session("2", "a", Some("1")),
                make_session("3", "b", Some("2")),
                make_session("4", "c", Some("3")),
                make_session("5", "d", Some("1")),
            ],
        };
        let rendered = tree.render_tree();
        assert_eq!(rendered.len(), 5);

        let lines: Vec<String> = rendered
            .iter()
            .map(|n| format_session_tree_line(n))
            .collect();
        assert_eq!(lines[0], ""); // root (depth 0)
        assert_eq!(lines[1], "├── "); // a (not last sibling)
        assert_eq!(lines[2], "│   └── "); // b (a has continuation, b is last child)
        assert_eq!(lines[3], "│       └── "); // c (a continues, b doesn't, c is last)
        assert_eq!(lines[4], "└── "); // d (last sibling of root)
    }

    #[test]
    fn test_tree_glyphs_multiple_roots() {
        let tree = SessionTree {
            sessions: vec![
                make_session("1", "main", None),
                make_session("2", "feature", None),
                make_session("3", "sub", Some("2")),
            ],
        };
        let rendered = tree.render_tree();
        assert_eq!(rendered.len(), 3);

        assert_eq!(format_session_tree_line(&rendered[0]), "");
        assert!(!rendered[0].is_last_sibling); // main is not last root
        assert_eq!(format_session_tree_line(&rendered[1]), "");
        assert!(rendered[1].is_last_sibling); // feature is last root
        assert_eq!(format_session_tree_line(&rendered[2]), "└── ");
        assert!(rendered[2].is_last_sibling);
    }

    #[test]
    fn test_tree_glyphs_empty_levels() {
        // root has one child with no grandchildren, but also a sibling with grandchildren
        // root
        // ├── a (no children — empty level)
        // └── b
        //     └── c
        let tree = SessionTree {
            sessions: vec![
                make_session("1", "root", None),
                make_session("2", "a", Some("1")),
                make_session("3", "b", Some("1")),
                make_session("4", "c", Some("3")),
            ],
        };
        let rendered = tree.render_tree();
        assert_eq!(rendered.len(), 4);

        let lines: Vec<String> = rendered
            .iter()
            .map(|n| format_session_tree_line(n))
            .collect();
        assert_eq!(lines[0], ""); // root
        assert_eq!(lines[1], "├── "); // a
        assert_eq!(lines[2], "└── "); // b
        assert_eq!(lines[3], "    └── "); // c (b is last sibling, no continuation)
    }
}
