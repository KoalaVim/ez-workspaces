//! Display-string formatting and small input parsers for the browser.

use colored::Colorize;

/// Format an ISO 8601 timestamp for display (e.g. "2h ago", "3d ago").
pub(crate) fn format_last_accessed(ts: &str) -> String {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) else {
        return ts.to_string();
    };
    let elapsed = chrono::Utc::now().signed_duration_since(dt);
    if elapsed.num_minutes() < 1 {
        format!("{}s ago", elapsed.num_seconds().max(0))
    } else if elapsed.num_hours() < 1 {
        format!("{}m ago", elapsed.num_minutes())
    } else if elapsed.num_hours() < 24 {
        format!("{}h ago", elapsed.num_hours())
    } else {
        format!("{}d ago", elapsed.num_days())
    }
}

/// Format a PR status indicator for display in the session picker.
pub(crate) fn format_pr_indicator(env: &std::collections::HashMap<String, String>) -> String {
    match (env.get("ez_pr_number"), env.get("ez_pr_status")) {
        (Some(num), Some(status)) => {
            let indicator = format!("[PR #{num} {status}]");
            match status.as_str() {
                "merged" => format!(" {}", indicator.magenta()),
                "closed" => format!(" {}", indicator.red()),
                _ => format!(" {}", indicator.green()),
            }
        }
        (Some(num), None) => format!(" {}", format!("[PR #{num}]").green()),
        _ => String::new(),
    }
}

/// Format a git branch indicator for display in the session picker.
pub(crate) fn format_branch_indicator(branch: Option<&str>) -> String {
    match branch {
        Some(name) => format!(" ({name})").truecolor(0, 120, 0).to_string(),
        None => String::new(),
    }
}

/// Shared display style for a repository row in any picker (drill-down,
/// repo view, owner view, etc.). `path` is the (collapse-tilded) path —
/// pass `None` when the surrounding context already shows it.
pub(crate) fn format_repo_display(
    name: &str,
    path: Option<&str>,
    branch: Option<&str>,
    labels: &[String],
) -> String {
    let mut parts = vec![name.bold().green().to_string()];
    if let Some(p) = path {
        parts.push(p.dimmed().to_string());
    }
    if let Some(b) = branch {
        parts.push(format!("[{b}]").cyan().to_string());
    }
    if !labels.is_empty() {
        parts.push(format!("[{}]", labels.join(",")).magenta().to_string());
    }
    parts.join(" ")
}

/// Parse a comma-separated label edit string.
///
/// - `foo, bar` → add `foo`, `bar`
/// - `-foo` → remove `foo`
///
/// Returns `(to_add, to_remove)`.
pub(crate) fn parse_label_input(input: &str) -> (Vec<String>, Vec<String>) {
    let mut add = Vec::new();
    let mut remove = Vec::new();
    for raw in input.split(',') {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(r) = token.strip_prefix('-') {
            let r = r.trim();
            if !r.is_empty() {
                remove.push(r.to_string());
            }
        } else {
            add.push(token.to_string());
        }
    }
    (add, remove)
}

#[cfg(test)]
mod tests {
    use super::parse_label_input;

    #[test]
    fn parses_add_and_remove() {
        let (a, r) = parse_label_input("foo, bar, -baz");
        assert_eq!(a, vec!["foo", "bar"]);
        assert_eq!(r, vec!["baz"]);
    }

    #[test]
    fn empty_input() {
        let (a, r) = parse_label_input("");
        assert!(a.is_empty());
        assert!(r.is_empty());
    }

    #[test]
    fn ignores_bare_dash() {
        let (a, r) = parse_label_input("-");
        assert!(a.is_empty());
        assert!(r.is_empty());
    }
}
