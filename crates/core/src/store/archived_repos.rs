//! Archived projects: a repo put away whole, with its tasks, kept in `archived-repos.json` in the data folder. Each
//! entry says when, which of its tasks were archived with it and which of its automations were turned off with it, so
//! that a restore brings back those and nothing else (actions/archive_project.rs).
use super::files::*;
use crate::err::Res;
use crate::json::*;
use std::path::PathBuf;

fn file() -> PathBuf { data_dir().join("archived-repos.json") }
/// Every archived repo by its name on the board: `at`, `tasks` (the ids archived with it), `automations` (the ids
/// turned off with it).
pub fn archived_repos() -> Obj { read_obj(&file()).unwrap_or_default() }
pub fn is_archived(repo: &str) -> bool { archived_repos().contains_key(repo) }
/// The one way the list changes, under its lock: `f` changes it in place.
pub fn change(f: impl FnOnce(&mut Obj)) -> Res<Obj> {
    with_lock_wait("archived-repos", 5000, || {
        let mut all = archived_repos();
        f(&mut all);
        write_json(&file(), &Value::Object(all.clone()))?;
        Ok(all)
    })
}
/// A new task in an archived repo is refused: it would stand alone on the board, its project put away.
pub fn refuse_archived(repo: &str) -> Res<()> {
    if is_archived(repo) { crate::fail!("{repo} is archived. herdr-projects restore --repo {repo} brings it back, with its tasks."); }
    Ok(())
}
