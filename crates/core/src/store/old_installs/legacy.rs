//! The old herdr-projects 0.2's projects, set aside: it kept them in ~/.herdr-projects (its "projects root"), the
//! name of this version's home folder, so the first run moves them whole to ~/.herdr-projects-legacy, as they are,
//! and the old version's own settings (~/.config/herdr-projects) and herdr's state folder for it beside them. Nothing
//! in them is changed, mixed with this version's files, or deleted. Its background ticker is stopped first.
use super::{legacy_name, Places, ID};
use crate::json::*;
use crate::store::files::read_json;
use crate::store::move_home::lstat;
use std::fs;
use std::path::{Path, PathBuf};

/// What the old version keeps at the top of its projects root, beside one folder per project.
const MARKS: [&str; 7] = [".ticker.lock", ".ticker.log", ".ticker.stop", ".machines.json", ".progress", ".renames", ".trash"];
/// What this version keeps in its home folder (docs/home-folder.md), and what an older one of it left there.
fn ours(name: &str) -> bool {
    matches!(name, "data" | "settings" | "reports" | "moved.json" | "set-aside" | "moving.lock") || name.starts_with("data-") || name.starts_with("review-") || (name.starts_with("closing-") && name.ends_with(".md"))
}
/// One of the old version's projects: a folder holding PROJECT.md.
fn project(dir: &Path) -> bool { lstat(&dir.join("PROJECT.md")).map(|m| m.is_file()).unwrap_or(false) }
fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}
/// What in this folder is the old version's: its own files at the top, and its projects.
pub fn old_entries(root: &Path) -> Vec<String> {
    if !lstat(root).map(|m| m.is_dir()).unwrap_or(false) { return vec![]; }
    names(root).into_iter().filter(|n| MARKS.contains(&n.as_str()) || project(&root.join(n))).collect()
}
/// The old version's settings folder holds its own files and none of this version's.
fn old_settings(dir: &Path) -> bool {
    lstat(dir).map(|m| m.is_dir()).unwrap_or(false) && !dir.join("config.json").exists() && ["config.toml", "owned.json", "approved-routines.json"].iter().any(|f| dir.join(f).exists())
}

/// Its ticker holds `.ticker.lock` while it runs. None: free (or no lock); Some(pid): held.
fn ticker_held(root: &Path) -> Option<i64> {
    let file = root.join(".ticker.lock");
    let f = fs::File::open(&file).ok()?;
    if crate::os::lock(&f, false) { crate::os::unlock(&f); return None; }
    Some(read_json(&file).map(|j| j.i("pid")).unwrap_or(0))
}
/// The ticker is asked to stop the old version's own way (a `.ticker.stop` file), then told to with a signal.
/// False: it still runs, and nothing is moved now.
fn stop_ticker(root: &Path, wait_ms: i64) -> bool {
    let stop = root.join(".ticker.stop");
    if ticker_held(root).is_none() { let _ = fs::remove_file(&stop); return true; } // a stop file left over: not set aside with them
    let _ = fs::write(&stop, "");
    let wait = |ms: i64| { let until = crate::clock::now() + ms; while ticker_held(root).is_some() { if crate::clock::now() >= until { return false; } crate::clock::sleep_ms(100); } true };
    let mut free = wait(wait_ms);
    if !free { if let Some(pid) = ticker_held(root).filter(|&p| p > 1) { crate::os::term(pid); free = wait(3000); } }
    if free { let _ = fs::remove_file(&stop); }
    free
}

/// Anything of the old version's to set aside.
pub fn due(p: &Places) -> bool { !old_entries(&p.to).is_empty() || old_settings(&p.old_config) || p.plugin_state.join(ID).join("handoff.json").exists() }

#[derive(Default, Debug)]
pub struct Aside { pub moved: Vec<(String, PathBuf, PathBuf)>, pub busy: bool }
/// Sets the old version's things aside, each whole or not at all. `busy`: its ticker would not stop, so its projects
/// stay where they are this time (and this version's files are not put beside them).
pub fn set_aside(p: &Places, wait_ms: i64) -> Aside {
    let mut out = Aside::default();
    let olds = old_entries(&p.to);
    let real_dir = lstat(&p.to).map(|m| m.is_dir()).unwrap_or(false);
    if real_dir && !olds.is_empty() {
        if !stop_ticker(&p.to, wait_ms) { out.busy = true; return out; }
        let to = legacy_name(&p.to);
        // All of it the old version's (or nobody's): the folder goes as one. This version's files there too: only the
        // old version's go, into a folder of their own.
        let whole = !names(&p.to).iter().any(|n| ours(n) && !olds.contains(n));
        let done = if whole { fs::rename(&p.to, &to).is_ok() } else {
            fs::create_dir_all(&to).is_ok() && olds.iter().all(|n| fs::rename(p.to.join(n), to.join(n)).is_ok())
        };
        if done { out.moved.push(("projects".into(), p.to.clone(), to)); } else { out.busy = true; return out; }
    }
    if old_settings(&p.old_config) {
        let to = legacy_name(&p.old_config);
        if fs::rename(&p.old_config, &to).is_ok() { out.moved.push(("settings".into(), p.old_config.clone(), to)); }
    }
    // herdr's state folder for the plugin: the old version's hand-over file is all it ever kept there.
    let state = p.plugin_state.join(ID);
    if state.join("handoff.json").exists() {
        let to = legacy_name(&state);
        if fs::rename(&state, &to).is_ok() { out.moved.push(("herdr state".into(), state, to)); }
    }
    out
}
