use crate::error::{EzError, Result};

use super::model::{Session, SessionId, SessionTree};

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

        loop {
            let session = match self.find_by_id(&current_id) {
                Some(s) => s,
                None => break,
            };
            match &session.parent_id {
                Some(pid) => {
                    if let Some(parent) = self.find_by_id(pid) {
                        result.push(parent);
                        current_id = pid.clone();
                    } else {
                        break;
                    }
                }
                None => break,
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

    /// Render as a depth-first tree: (depth, session) pairs.
    pub fn render_tree(&self) -> Vec<(usize, &Session)> {
        let mut result = Vec::new();
        let roots = self.roots();
        for root in roots {
            self.render_subtree(root, 0, &mut result);
        }
        result
    }

    fn render_subtree<'a>(
        &'a self,
        session: &'a Session,
        depth: usize,
        result: &mut Vec<(usize, &'a Session)>,
    ) {
        result.push((depth, session));
        for child in self.children(&session.id) {
            self.render_subtree(child, depth + 1, result);
        }
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
            created_at: Utc::now(),
            is_default: false,
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
        assert_eq!(rendered[0].0, 0); // main at depth 0
        assert_eq!(rendered[1].0, 0); // feature at depth 0
        assert_eq!(rendered[2].0, 1); // sub at depth 1
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
}
