//! The copy of the data an older (Node) version wrote: the first run of this version keeps the data folder as it was,
//! beside it (`~/.herdr-projects/data-0.2.0`), before anything in it is changed. The data's form is the same, so
//! nothing is converted; the copy is only there to go back with. Made once: skipped when it is there. Its one line is
//! said as the move's is (store/move_home.rs): on the terminal, or on the board when the first run was a hook.
use super::files::*;
use super::move_home::{add_line, env_now, launcher_target, move_note, move_plan, move_told, OLD_NAME};
use crate::json::*;
use std::fs;
use std::path::{Path, PathBuf};

/// The first version that is this program (Rust); every one before it was Node.
pub const FIRST_NATIVE: &str = "0.3.0";
/// The version that wrote this data folder, when it was an older one: the version its keeper wrote down, or "node"
/// when only its launcher says so: `exec node '…/herdr-superwork.js' "$@"` in `bin/herdr-superwork`, by that version's
/// name, while this version has not put its own launcher beside it. None: this version's data, or none.
pub fn older_writer(data: &Path) -> Option<String> {
    let kept = read_obj(&data.join("keeper.json")).map(|o| o.t("version")).unwrap_or_default();
    if !kept.is_empty() && crate::util::newer(FIRST_NATIVE, &kept) { return Some(kept); }
    let launcher = ["herdr-projects", OLD_NAME].map(|n| data.join("bin").join(n)).into_iter().find(|f| f.exists())?;
    let node = read_text(&launcher).and_then(|t| t.lines().find_map(launcher_target)).map(|p| p.ends_with(".js")).unwrap_or(false);
    node.then(|| "node".to_string())
}
/// The copy's place: beside the data folder, named after the version that wrote it.
pub fn copy_place(data: &Path, version: &str) -> PathBuf { PathBuf::from(format!("{}-{version}", data.display())) }
/// Files, folders and links as they are (a link stays a link); anything else (a socket) is left out.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    let m = fs::symlink_metadata(from)?;
    if m.file_type().is_symlink() { return crate::os::symlink(&fs::read_link(from)?, to); }
    if m.is_dir() {
        fs::create_dir(to)?;
        for e in fs::read_dir(from)? { let e = e?; copy_tree(&e.path(), &to.join(e.file_name()))?; }
        return Ok(());
    }
    if m.is_file() { fs::copy(from, to)?; }
    Ok(())
}
/// Keeps the copy when it is due. Some(the line for the human) when it was made now. It is written under a spare name
/// and renamed into place whole, so a cut-off run leaves no half copy under the real name; the next run makes it.
pub fn keep_old_copy(data: &Path) -> Result<Option<String>, String> {
    let Some(v) = older_writer(data) else { return Ok(None) };
    let to = copy_place(data, &v);
    if fs::symlink_metadata(&to).is_ok() { return Ok(None); }
    let name = to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let dir = to.parent().map(Path::to_path_buf).unwrap_or_default();
    // A spare copy a cut-off run left behind (its process is gone) is removed first.
    for e in fs::read_dir(&dir).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if let Some(pid) = n.strip_prefix(&format!("{name}.copying-")).and_then(|p| p.parse::<i64>().ok()) { if !pid_alive(pid) { let _ = fs::remove_dir_all(e.path()); } }
    }
    let spare = dir.join(format!("{name}.copying-{}", std::process::id()));
    let made = copy_tree(data, &spare).and_then(|_| fs::rename(&spare, &to));
    if let Err(e) = made {
        let _ = fs::remove_dir_all(&spare);
        if fs::symlink_metadata(&to).is_ok() { return Ok(None); } // another start made it at the same time
        return Err(format!("Could not keep a copy of {} before this version uses it: {e}", data.display()));
    }
    let older = if v == "node" { "the older (Node) version".to_string() } else { format!("version {v}") };
    Ok(Some(format!("herdr-projects {VERSION} uses the data {older} kept, unchanged; a copy of it as it was is in {}.", short(&to))))
}
fn short(p: &Path) -> String {
    let home = user_home();
    match p.strip_prefix(&home) { Ok(rest) if home != Path::new("/") => format!("~/{}", rest.display()), _ => p.display().to_string() }
}
/// Commands run with nobody reading their error output (as in move_home.rs): the line waits for a human.
const QUIET: [&str; 8] = ["board", "hook", "turn-ended", "keeper", "keeper-work", "run", "serve", "remote"];
/// At the start of every run, after the move into the home folder: the copy, and its one line, added to the move's
/// own line so that the board shows it too when the first run was a hook.
pub fn copy_at_start(cmd: &str) {
    if std::env::var("HERDR_PROJECTS_INLINE").as_deref() == Ok("1") { return; }
    let (home, env) = (user_home(), env_now());
    let line = match keep_old_copy(&data_dir()) { Ok(Some(l)) => l, Ok(None) => String::new(), Err(e) => e };
    if !line.is_empty() {
        let file = move_plan(&home, &env).file;
        let mut rec = read_obj(&file).unwrap_or_default();
        if rec.at("told") == &json!(false) && rec.t("line").contains(&line) { return; } // a copy that failed again: said once
        add_line(&mut rec, &line);
        let _ = write_json(&file, &Value::Object(rec));
    }
    if QUIET.contains(&cmd) || !crate::os::is_tty(2) { return; }
    let said = move_note(&home, &env);
    if !said.is_empty() { eprintln!("{said}"); move_told(&home, &env); }
}
