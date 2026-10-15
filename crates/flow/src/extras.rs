//! The things the plugin writes outside its own folders, each consented and removable. They are:
//!   1. a small how-to in each installed agent program's own skills folder (always written; `remove-extras` takes it
//!      out for good), so any chat knows the board without being told (its words: extras/skill_text.rs);
//!   2. on a yes at the board's first run, marked blocks in herdr's config.toml: a key to open the board and the
//!      sidebar rows that show the plugin's labels (extras/config.rs);
//!   3. on a yes, with the herdr web UI plugin installed: where that web UI's shared link (`tailscale serve`) points,
//!      so the Board shows in it (webui/link.rs does it and writes down what it changed).
//! `remove-extras` takes all of it out again. The plugin never edits a file, a block or a link it did not write.
pub mod config;
pub mod skill_text;
pub use config::{add_block, block_text, board_key, board_presses, check_key, herdr_config, offers, refresh_sidebar, remove_blocks, remove_popups};

use crate::agents::{adapter::skill_dir, installed::installed, kinds::KIND_NAMES};
use crate::json::*;
use crate::runner::inline;
use crate::store::files::{data_dir, read_obj, with_lock_wait, write_json};
use std::path::{Path, PathBuf};

fn state_file() -> PathBuf { data_dir().join("extras.json") }
pub fn extras() -> Obj { read_obj(&state_file()).unwrap_or_default() }
pub fn save(patch: Value) -> Obj { let next = merged(&extras(), &obj(patch)); let _ = write_json(&state_file(), &Value::Object(next.clone())); next }
/// Written down beside the answers: what the first run was told, and what else the plugin changed outside its folders.
pub fn answer(patch: Value) -> Obj { let mut p = obj(patch); p.put("asked", true); save(Value::Object(p)) }
pub fn note(patch: Value) -> Obj { save(patch) }

// ---- 1. the how-to ---------------------------------------------------------------------------------------------------
const NAME: &str = "herdr-projects";
/// Each installed agent program's own user-level skills folder (a folder per skill holding SKILL.md). The plugin owns
/// one folder there, named for itself, and records every path it wrote.
fn skill_targets() -> Vec<PathBuf> {
    if let Ok(dirs) = std::env::var("HERDR_PROJECTS_SKILL_DIRS") { return dirs.split(':').filter(|d| !d.is_empty()).map(|d| Path::new(d).join(NAME)).collect(); }
    KIND_NAMES.iter().filter_map(|k| skill_dir(k).filter(|_| installed(k))).map(|d| d.join(NAME)).collect() // only a program whose skills folder the board knows
}
pub fn sync_skill() -> Vec<String> { with_lock_wait("extras", 15000, || Ok(sync_skill0())).unwrap_or_default() }
fn sync_skill0() -> Vec<String> {
    if inline() && std::env::var("HERDR_PROJECTS_SKILL_DIRS").is_err() { return vec![]; } // tests never write into real agent folders
    if extras().b("skillOff") { remove_skill0(); return vec![]; } // taken out with remove-extras: not written back
    let mut wrote: Vec<String> = strings(extras().at("skillPaths"));
    let text = skill_text::skill_text(&data_dir().join("bin").join(NAME).display().to_string());
    for dir in skill_targets() {
        let d = dir.display().to_string();
        let file = dir.join("SKILL.md");
        if dir.exists() && !wrote.contains(&d) { continue; } // a folder of that name that is not ours: left alone
        if std::fs::read_to_string(&file).ok().as_deref() == Some(text.as_str()) { if !wrote.contains(&d) { wrote.push(d); } continue; }
        if std::fs::create_dir_all(&dir).is_ok() && std::fs::write(&file, &text).is_ok() && !wrote.contains(&d) { wrote.push(d); }
    }
    save(json!({ "skillPaths": wrote }));
    wrote
}
/// Removes exactly what was written: the recorded folders, and only while they still hold just our file.
pub fn remove_skill() { let _ = with_lock_wait("extras", 15000, || { remove_skill0(); Ok(()) }); }
fn remove_skill0() {
    let paths = strings(extras().at("skillPaths"));
    let mut gone = vec![];
    for dir in &paths {
        match std::fs::read_dir(dir) {
            Ok(rd) => { let names: Vec<String> = rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect(); if names.iter().all(|n| n == "SKILL.md") { let _ = std::fs::remove_dir_all(dir); gone.push(dir.clone()); } }
            Err(_) => gone.push(dir.clone()),
        }
    }
    save(json!({ "skillPaths": paths.into_iter().filter(|d| !gone.contains(d)).collect::<Vec<_>>() }));
}

/// Everything the plugin wrote outside its own folders, taken out, and nothing written back at the next start. herdr
/// gives a plugin no uninstall hook, so docs/install.md says to run this before unlinking.
pub fn remove_extras() -> Value {
    let blocks = remove_blocks(None);
    remove_skill();
    save(json!({ "asked": true, "key": "no", "sidebarAnswer": "no", "sidebar": false, "skillOff": true }));
    json!({ "blocks": blocks, "skill": "off" })
}
/// 3. The web UI's shared link goes back to the web UI, and the front door stops (webui/command.rs).
pub fn remove_web_ui() -> Value { crate::webui::command::leave() }
