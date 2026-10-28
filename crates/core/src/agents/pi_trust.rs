//! Marking a folder as trusted in pi's own trust.json (#413), as pi 1.4 keeps it (its docs/security.md): one object of
//! folder paths, each `true` or `false` (or `null`, no decision), and the closest folder from the one pi starts in up
//! that has a decision wins. pi's own `false` there is its answer and is left as it is; a file that does not read, or
//! that pi itself is writing (its `trust.json.lock`), is not touched.
use super::homes::pi_home;
use crate::json::*;
use std::fs;
use std::path::{Path, PathBuf};

/// pi's decision for a folder, from its file: the closest folder with `true` or `false`, from the folder up.
fn decided(file: &Obj, folder: &Path) -> Option<bool> { folder.ancestors().find_map(|d| file.get(&d.display().to_string()).and_then(Value::as_bool)) }

/// Trusted in pi's trust.json, adding the folder (its real path, as pi looks it up) when no folder above it decided.
/// False: its folder dialog shows.
pub fn trust(folder: &Path) -> bool {
    let folder = fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf());
    let file = pi_home().join("trust.json");
    let Some(all) = read(&file) else { return false };
    if let Some(yes) = decided(&all, &folder) { return yes; }
    // pi's own lock while it writes the file: a folder beside it (proper-lockfile).
    let lock = PathBuf::from(format!("{}.lock", file.display()));
    if let Some(d) = file.parent() { let _ = fs::create_dir_all(d); }
    if !locked(&lock) { return false; }
    let done = add(&file, &folder);
    let _ = fs::remove_dir(&lock);
    done
}
/// Taken as pi takes it: tried 10 times, 20 ms apart, while another holds it (pi, or a second start of the board's);
/// one left behind by a pi that quit is stale after 10 s.
fn locked(lock: &Path) -> bool {
    for _ in 0..10 {
        if fs::create_dir(lock).is_ok() { return true; }
        let old = fs::metadata(lock).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).map(|e| e.as_secs() >= 10).unwrap_or(false);
        if old { let _ = fs::remove_dir(lock); continue; }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    false
}
/// The file as pi reads it: None when it does not read as an object.
fn read(file: &Path) -> Option<Obj> {
    match fs::read_to_string(file) {
        Ok(t) => match serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')) { Ok(Value::Object(o)) => Some(o), _ => None },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(Obj::new()),
        Err(_) => None,
    }
}
/// Read again under the lock, then written whole, its keys sorted as pi writes them.
fn add(file: &Path, folder: &Path) -> bool {
    let Some(mut all) = read(file) else { return false };
    if let Some(yes) = decided(&all, folder) { return yes; }
    all.insert(folder.display().to_string(), Value::Bool(true));
    let sorted: std::collections::BTreeMap<&String, &Value> = all.iter().collect();
    let tmp = PathBuf::from(format!("{}.{}.sw.tmp", file.display(), std::process::id()));
    let done = super::adapter::write_private(&tmp, &format!("{}\n", serde_json::to_string_pretty(&sorted).unwrap_or_default())).is_ok() && fs::rename(&tmp, file).is_ok();
    let _ = fs::remove_file(&tmp);
    done
}
