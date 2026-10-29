//! A task's own places and its herdr: the closing note, the review folder, whether an agent is on it. What an agent
//! writes for the board goes in the task's own folder inside the board's data folder, never into the folder it works
//! in (docs/home-folder.md). `as`: the chosen attempt of several is the task from then on, and its agent still writes
//! where it was told to when it started as that attempt.
use crate::herdr::Herdr;
use crate::json::*;
use crate::page::{self, Kind};
use crate::store::files::dir_of;
use crate::store::repos::cwd_of;
use std::path::PathBuf;

pub fn note_file(t: &Obj) -> PathBuf { dir_of(if t.b("as") { t.s("as") } else { t.s("id") }).join("closing-note.md") }
pub fn review_dir(t: &Obj) -> PathBuf { dir_of(t.s("id")).join("review") }
/// The board's folder in the folder an agent works in: only the files sent from the web board go there (webui/attach.rs).
pub const RULE_DIR: &str = ".herdr-projects";
/// Where older versions told agents to write: `<working folder>/.herdr-superwork/closing-<n>.md`, by that version's
/// name. A task first started by this version (`ownNote`) is never looked for there. Nothing is written there any more.
pub const OLD_RULE_DIR: &str = ".herdr-superwork";
pub fn old_note_file(t: &Obj) -> PathBuf { cwd_of(t).join(OLD_RULE_DIR).join(format!("closing-{}.md", t.t("n"))) }
fn stat_of(file: &PathBuf) -> Option<(f64, String)> {
    let m = std::fs::metadata(file).ok()?;
    let mtime = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs_f64() * 1000.0;
    Some((mtime, std::fs::read_to_string(file).ok()?.trim().to_string()))
}
/// The closing note, wherever its agent was told to write it: the newer one when both places have one. (mtime, text)
pub fn note_stat(t: &Obj) -> Option<(f64, String)> {
    let now = stat_of(&note_file(t));
    let old = if t.b("ownNote") { None } else { stat_of(&old_note_file(t)) };
    match (old, now) { (Some(o), Some(n)) => Some(if o.0 > n.0 { o } else { n }), (Some(o), None) => Some(o), (None, n) => n }
}
/// A task that was cleaned up leaves no note behind. The old folder goes only when it is empty.
pub fn clear_notes(t: &Obj) {
    let mut files = vec![note_file(t), review_dir(t)];
    if !t.b("ownNote") { files.push(old_note_file(t)); }
    for f in files { let _ = std::fs::remove_dir_all(&f); let _ = std::fs::remove_file(&f); }
    let _ = std::fs::remove_dir(cwd_of(t).join(OLD_RULE_DIR));
}
/// The closing summary as a readable page and its link, when there is one. The note itself is never changed.
pub fn closing_view(t: &Obj, base: impl FnOnce() -> String) -> Option<Value> {
    let note = note_file(t);
    if !note.is_file() { return None; }
    let dir = dir_of(t.s("id")).join("artifacts").join("closing");
    let title = if t.b("title") { t.t("title") } else { t.t("id") };
    let make_page = t.s("stage") == "done";
    if make_page { let _ = std::fs::create_dir_all(&dir); let _ = page::ensure_page(&dir, "../../closing-note.md", &title, Kind::Closing); }
    let root = base();
    let page = make_page && dir.join("index.html").is_file();
    let entry = if page { "closing/index.html" } else { "closing-note.md" };
    let link = if root.is_empty() { if page { dir.join("index.html").display().to_string() } else { note.display().to_string() } } else { format!("{root}/{entry}") };
    Some(json!({ "file": if page { "index.html" } else { "closing-note.md" }, "link": link, "source": "closing-note.md" }))
}
pub fn is_agent_task(t: &Obj) -> bool { t.at("link").b("pane") }
/// The herdr session a task's agents live in.
pub fn h(t: &Obj) -> Herdr { Herdr::new(t.at("link").so("socket").map(String::from)) }
