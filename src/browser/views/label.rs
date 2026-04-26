use std::collections::BTreeMap;
use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::paths;
use crate::plugin;
use crate::repo;
use crate::session;

use super::super::selector::{ActionResult, InteractiveSelector, SelectItem};
use super::super::{browse_repo, write_cd_target};
use super::{match_view_switch, view_header, view_switch_keys, Outcome, ViewMode};

#[derive(Clone)]
enum LabeledItem {
    Repo(repo::model::RepoEntry),
    Session(Box<LabeledSession>),
}

#[derive(Clone)]
struct LabeledSession {
    repo: repo::model::RepoEntry,
    session: session::model::Session,
}

pub(super) fn run(
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
) -> Result<Outcome> {
    let index = repo::store::load_index()?;
    let mut by_label: BTreeMap<String, Vec<LabeledItem>> = BTreeMap::new();

    for entry in &index.repos {
        let meta = repo::store::load_repo_meta(&entry.id).unwrap_or_default();
        for label in &meta.labels {
            by_label
                .entry(label.clone())
                .or_default()
                .push(LabeledItem::Repo(entry.clone()));
        }
        if let Ok(tree) = session::store::load_sessions(&entry.id) {
            for session in &tree.sessions {
                for label in &session.labels {
                    by_label
                        .entry(label.clone())
                        .or_default()
                        .push(LabeledItem::Session(Box::new(LabeledSession {
                            repo: entry.clone(),
                            session: session.clone(),
                        })));
                }
            }
        }
    }

    if by_label.is_empty() {
        println!(
            "{}",
            "No labels found. Use `ez repo label add` or `ez session label add`.".yellow()
        );
        return Ok(Outcome::Done);
    }

    let plugin_views = plugin::collect_plugin_views("label", config).unwrap_or_default();

    let labels: Vec<(String, Vec<LabeledItem>)> = by_label.into_iter().collect();
    let items: Vec<SelectItem> = labels
        .iter()
        .map(|(label, items)| SelectItem {
            display: format!(
                "{} {}",
                label.bold().magenta(),
                format!(
                    "({} item{})",
                    items.len(),
                    if items.len() == 1 { "" } else { "s" }
                )
                .dimmed()
            ),
            value: label.clone(),
        })
        .collect();

    let header = view_header("label", &config.keybinds, &plugin_views);

    let action = selector.select_with_actions(
        &items,
        "label",
        None,
        &view_switch_keys(&config.keybinds, &plugin_views),
        Some(&header),
    )?;

    let label_idx = match action {
        ActionResult::Cancel => return Ok(Outcome::Done),
        ActionResult::Action(key, _) => {
            return match match_view_switch(&config.keybinds, &plugin_views, &key) {
                Some(next) => Ok(Outcome::Switch(next)),
                None => Ok(Outcome::Done),
            }
        }
        ActionResult::Select(idx) => idx,
    };

    let (_label, tagged) = &labels[label_idx];
    let sub_items: Vec<SelectItem> = tagged
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let display = match item {
                LabeledItem::Repo(r) => format!(
                    "{} {} {}",
                    "●".green(),
                    r.name.bold().green(),
                    paths::collapse_tilde(&r.path.to_string_lossy()).dimmed(),
                ),
                LabeledItem::Session(b) => format!(
                    "{} {} {}{}",
                    "◆".yellow(),
                    b.session.name.bold().yellow(),
                    format!("({})", b.repo.name).dimmed(),
                    b.session
                        .path
                        .as_ref()
                        .map(|p| format!(" → {}", p.display()).dimmed().to_string())
                        .unwrap_or_default(),
                ),
            };
            SelectItem {
                display,
                value: i.to_string(),
            }
        })
        .collect();

    let sub_action = selector.select_with_actions(
        &sub_items,
        "tagged",
        None,
        &view_switch_keys(&config.keybinds, &plugin_views),
        Some(&header),
    )?;

    match sub_action {
        ActionResult::Cancel => Ok(Outcome::Switch(ViewMode::Label)),
        ActionResult::Action(key, _) => match match_view_switch(&config.keybinds, &plugin_views, &key) {
            Some(next) => Ok(Outcome::Switch(next)),
            None => Ok(Outcome::Done),
        },
        ActionResult::Select(idx) => match &tagged[idx] {
            LabeledItem::Repo(r) => {
                browse_repo(&r.path, selector, cd_file, post_cmd_file, config)?;
                Ok(Outcome::Done)
            }
            LabeledItem::Session(b) => {
                let target = b
                    .session
                    .path
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| b.repo.path.clone());
                write_cd_target(cd_file, &target)?;
                Ok(Outcome::Done)
            }
        },
    }
}
