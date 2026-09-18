//! Inbox items: events the ticker leaves for the coordinator.

use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::project::{self, Project};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Item {
    pub id: String,
    pub kind: String,
    pub subject: String,
    pub created: String,
    pub summary: String,
    /// Empty except for `routine` items.
    #[serde(skip)]
    pub body: String,
}

fn inbox_dir(project: &Project) -> PathBuf {
    project.dir().join("inbox")
}

fn parse(text: &str) -> Option<Item> {
    let rest = text.strip_prefix("+++\n")?;
    let (front, body) = rest.split_once("\n+++\n").or_else(|| Some((rest.strip_suffix("\n+++")?, "")))?;
    let mut item: Item = toml::from_str(front).ok()?;
    item.body = body.trim_matches('\n').to_string();
    Some(item)
}

/// File-name-safe form of a subject (a thread id, routine name, machine label).
pub fn safe_subject(subject: &str) -> String {
    let cleaned: String = subject
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c.to_ascii_lowercase() } else { '-' })
        .take(40)
        .collect();
    let cleaned = cleaned.trim_matches('-').to_string();
    if cleaned.is_empty() { "item".to_string() } else { cleaned }
}

/// Writes one item. The id is `<UTC timestamp>-<kind>-<subject>-<n>`, where
/// `<n>` is a counter allocated under the project lock, so two events in one
/// tick never share a name. `body` is empty except for `routine` items.
pub fn write(project: &Project, kind: &str, subject: &str, summary: &str, body: &str) -> Result<String> {
    let _lock = project.lock()?;
    let counter_path = project.state_dir().join("inbox-counter.json");
    let n: u64 = project::read_json::<u64>(&counter_path).unwrap_or(0) + 1;
    project::write_json(&counter_path, &n)?;
    let stamp = jiff::Timestamp::now().strftime("%Y%m%dT%H%M%SZ").to_string();
    let id = format!("{stamp}-{kind}-{}-{n}", safe_subject(subject));
    let item = Item {
        id: id.clone(),
        kind: kind.to_string(),
        subject: subject.to_string(),
        created: project::now(),
        // One line, no control characters: summaries are printed in the digest.
        summary: summary.chars().map(|c| if c.is_control() { ' ' } else { c }).collect(),
        body: String::new(),
    };
    let mut text = format!("+++\n{}+++\n", toml::to_string(&item)?);
    if !body.is_empty() {
        text.push('\n');
        text.push_str(body.trim_end());
        text.push('\n');
    }
    project::write_atomic(&inbox_dir(project).join(format!("{id}.md")), text.as_bytes())?;
    Ok(id)
}

/// Deletes handled items older than `days`.
pub fn prune_done(project: &Project, days: u64) {
    let Ok(entries) = std::fs::read_dir(inbox_dir(project).join("done")) else {
        return;
    };
    let limit = std::time::Duration::from_secs(days * 24 * 3600);
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > limit);
        if old {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Unhandled items, oldest first (ids start with a UTC timestamp).
pub fn unhandled(project: &Project) -> Vec<Item> {
    let Ok(entries) = std::fs::read_dir(inbox_dir(project)) else {
        return Vec::new();
    };
    let mut items: Vec<Item> = entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".md"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|text| parse(&text))
        .collect();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    items
}

pub fn seen(project: &Project) -> BTreeSet<String> {
    project::read_json(&project.state_dir().join("inbox-seen.json")).unwrap_or_default()
}

/// Records that `context` showed these items, so they are nudged once only.
pub fn mark_seen(project: &Project, ids: &[String]) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let _lock = project.lock()?;
    let mut all = seen(project);
    all.extend(ids.iter().cloned());
    // Ids of items that no longer exist are dropped so the file stays small.
    let live: BTreeSet<String> = unhandled(project).into_iter().map(|i| i.id).collect();
    all.retain(|id| live.contains(id));
    project::write_json(&project.state_dir().join("inbox-seen.json"), &all)
}

/// An item id is also a file name, so it is checked before any path is built.
fn validate_id(id: &str) -> Result<()> {
    let ok = !id.is_empty()
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !ok || id.contains("..") {
        bail!("`{id}` is not an inbox item id");
    }
    Ok(())
}

/// Moves items to `inbox/done/`. Returns how many moved.
pub fn done(project: &Project, ids: &[String], all: bool) -> Result<usize> {
    let ids: Vec<String> = if all {
        unhandled(project).into_iter().map(|i| i.id).collect()
    } else {
        ids.to_vec()
    };
    for id in &ids {
        validate_id(id)?;
    }
    let _lock = project.lock()?;
    let dir = inbox_dir(project);
    let mut moved = 0;
    for id in &ids {
        let from = dir.join(format!("{id}.md"));
        if !from.is_file() {
            eprintln!("no unhandled item `{id}`");
            continue;
        }
        std::fs::rename(&from, dir.join("done").join(format!("{id}.md")))?;
        moved += 1;
    }
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_item(project: &Project, id: &str, body: &str) {
        let text = format!(
            "+++\nid = \"{id}\"\nkind = \"routine\"\nsubject = \"r\"\ncreated = \"2026-09-17T00:00:00Z\"\nsummary = \"s\"\n+++\n{body}"
        );
        std::fs::write(inbox_dir(project).join(format!("{id}.md")), text).unwrap();
    }

    #[test]
    fn lists_marks_seen_and_moves_to_done() {
        let root = tempfile::tempdir().unwrap();
        let project = project::create(root.path(), "demo", "", vec![]).unwrap();
        write_item(&project, "20260917T000002Z-routine-r-2", "\nbody text\n");
        write_item(&project, "20260917T000001Z-routine-r-1", "");
        let items = unhandled(&project);
        assert_eq!(items.len(), 2);
        assert!(items[0].id.ends_with("-1"));
        assert_eq!(items[1].body, "body text");

        mark_seen(&project, &[items[0].id.clone()]).unwrap();
        assert_eq!(seen(&project).len(), 1);

        assert_eq!(done(&project, &[items[0].id.clone()], false).unwrap(), 1);
        assert_eq!(unhandled(&project).len(), 1);
        assert!(inbox_dir(&project).join("done").join(format!("{}.md", items[0].id)).is_file());
        assert_eq!(done(&project, &[], true).unwrap(), 1);
        assert!(unhandled(&project).is_empty());
    }

    #[test]
    fn two_events_in_one_tick_get_two_items() {
        let root = tempfile::tempdir().unwrap();
        let project = project::create(root.path(), "demo", "", vec![]).unwrap();
        let a = write(&project, "thread-state", "t-0001", "first", "").unwrap();
        let b = write(&project, "thread-state", "t-0001", "second\nline", "").unwrap();
        assert_ne!(a, b);
        assert!(a.ends_with("-thread-state-t-0001-1"), "{a}");
        assert!(b.ends_with("-thread-state-t-0001-2"), "{b}");
        let items = unhandled(&project);
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].summary, "second line");
        assert!(items.iter().all(|i| i.body.is_empty()));
        // A written item can be marked done by its id.
        assert_eq!(done(&project, &[a], false).unwrap(), 1);
    }

    #[test]
    fn routine_items_carry_a_body_and_subjects_are_made_file_safe() {
        let root = tempfile::tempdir().unwrap();
        let project = project::create(root.path(), "demo", "", vec![]).unwrap();
        let id = write(&project, "outage", "Elias MacBook/../x", "down", "").unwrap();
        assert!(id.contains("-outage-elias-macbook----x-"), "{id}");
        write(&project, "routine", "nightly", "due", "Check the build.\n\n```\nout\n```").unwrap();
        let routine = unhandled(&project).into_iter().find(|i| i.kind == "routine").unwrap();
        assert!(routine.body.starts_with("Check the build."));
        assert!(routine.body.ends_with("```"));
    }

    #[test]
    fn hostile_ids_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let project = project::create(root.path(), "demo", "", vec![]).unwrap();
        for bad in ["../PROJECT", "a/b", "", ".hidden", "x..y"] {
            assert!(done(&project, &[bad.to_string()], false).is_err(), "{bad}");
        }
    }
}
