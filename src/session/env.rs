use std::collections::BTreeMap;

use colored::Colorize;

use super::current;
use super::store;
use crate::cli::SessionEnvCommand;
use crate::error::{EzError, Result};
use crate::repo;
use crate::repo::model::RepoEntry;

pub(super) fn dispatch_env(cmd: SessionEnvCommand) -> Result<()> {
    match cmd {
        SessionEnvCommand::Set {
            key,
            value,
            session,
            repo,
        } => session_env_set(repo.as_deref(), session.as_deref(), &key, &value),
        SessionEnvCommand::List {
            session,
            repo,
            json,
        } => session_env_list(repo.as_deref(), session.as_deref(), json),
        SessionEnvCommand::Unset { key, session, repo } => {
            session_env_unset(repo.as_deref(), session.as_deref(), &key)
        }
    }
}

struct EnvTarget {
    repo_entry: RepoEntry,
    session_id: String,
    session_name: String,
}

fn resolve_env_target(repo_name: Option<&str>, session_name: Option<&str>) -> Result<EnvTarget> {
    match session_name {
        Some(name) => {
            let repo_entry = repo::resolve_repo(repo_name)?;
            let tree = store::load_sessions(&repo_entry.id)?;
            let session = tree
                .find_by_name(name)
                .ok_or_else(|| EzError::SessionNotFound(name.into()))?;
            Ok(EnvTarget {
                repo_entry,
                session_id: session.id.clone(),
                session_name: session.name.clone(),
            })
        }
        None => match current::resolve_current_session(repo_name) {
            Ok(target) => Ok(EnvTarget {
                repo_entry: target.repo_entry,
                session_id: target.session.id.clone(),
                session_name: target.session.name.clone(),
            }),
            Err(EzError::SessionNotFound(_)) => Err(EzError::Config(
                "No current session detected. Use --session <name> to specify a session.".into(),
            )),
            Err(e) => Err(e),
        },
    }
}

fn session_env_set(
    repo_name: Option<&str>,
    session_name: Option<&str>,
    key: &str,
    value: &str,
) -> Result<()> {
    if key.is_empty() {
        return Err(EzError::Config(
            "Environment variable key cannot be empty".into(),
        ));
    }

    let target = resolve_env_target(repo_name, session_name)?;
    let mut tree = store::load_sessions(&target.repo_entry.id)?;
    let session = tree
        .sessions
        .iter_mut()
        .find(|s| s.id == target.session_id)
        .ok_or_else(|| EzError::SessionNotFound(target.session_name.clone()))?;

    log::debug!(
        "session_env_set: {}={} on session '{}' ({})",
        key,
        value,
        session.name,
        session.id
    );
    session.env.insert(key.to_string(), value.to_string());
    store::save_sessions(&target.repo_entry.id, &tree)?;

    println!(
        "{} {} on session {}",
        "Set".green(),
        key.bold(),
        target.session_name.bold()
    );
    Ok(())
}

fn session_env_unset(repo_name: Option<&str>, session_name: Option<&str>, key: &str) -> Result<()> {
    let target = resolve_env_target(repo_name, session_name)?;
    let mut tree = store::load_sessions(&target.repo_entry.id)?;
    let session = tree
        .sessions
        .iter_mut()
        .find(|s| s.id == target.session_id)
        .ok_or_else(|| EzError::SessionNotFound(target.session_name.clone()))?;

    log::debug!(
        "session_env_unset: key '{}' on session '{}' ({})",
        key,
        session.name,
        session.id
    );
    if session.env.remove(key).is_none() {
        log::debug!(
            "session_env_unset: key '{}' was not set on session '{}'",
            key,
            target.session_name
        );
        return Ok(());
    }

    store::save_sessions(&target.repo_entry.id, &tree)?;
    println!(
        "{} {} on session {}",
        "Unset".green(),
        key.bold(),
        target.session_name.bold()
    );
    Ok(())
}

fn session_env_list(repo_name: Option<&str>, session_name: Option<&str>, json: bool) -> Result<()> {
    let target = resolve_env_target(repo_name, session_name)?;
    let tree = store::load_sessions(&target.repo_entry.id)?;
    let session = tree
        .sessions
        .iter()
        .find(|s| s.id == target.session_id)
        .ok_or_else(|| EzError::SessionNotFound(target.session_name.clone()))?;

    log::debug!(
        "session_env_list: {} var(s) on session '{}' ({})",
        session.env.len(),
        session.name,
        session.id
    );

    let env: BTreeMap<&String, &String> = session.env.iter().collect();
    if json {
        println!("{}", serde_json::to_string(&env)?);
        return Ok(());
    }

    for (key, value) in env {
        println!("{}{}{}", key.cyan(), "=".dimmed(), value);
    }
    Ok(())
}
