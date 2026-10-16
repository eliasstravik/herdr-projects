//! The one-time move into the one home folder: what an older version kept in ~/.local/state, ~/.config and
//! ~/.local/share is moved into ~/.herdr-projects by the first run of this code, by itself, and each old place becomes
//! a link to the new one, so whatever still runs from the old paths carries on with the same files.
//! docs/home-folder.md has the whole story. Every step is whole or not done, so a run that is cut off is finished by
//! the next one; no step leaves an old path missing, not for a moment; no data is deleted or written over.
use super::files::*;
use crate::clock::now;
use crate::json::*;
use std::fs;
use std::path::{Path, PathBuf};

pub fn lstat(p: &Path) -> Option<fs::Metadata> { fs::symlink_metadata(p).ok() }
pub fn real(p: &Path) -> PathBuf { fs::canonicalize(p).unwrap_or_else(|_| absolute(p)) }
pub fn short(p: &Path, home: &Path) -> String {
    if p == home { "~".into() } else if let Ok(rest) = p.strip_prefix(home) { format!("~/{}", rest.display()) } else { p.display().to_string() }
}
#[derive(Clone)]
pub struct Part { pub what: &'static str, pub from: PathBuf, pub to: PathBuf, pub key: &'static str }
pub struct Plan { pub home: PathBuf, pub file: PathBuf, pub aside: PathBuf, pub parts: Vec<Part> }
pub type Env = std::collections::HashMap<String, String>;
/// The name those older versions went by, which is all that finds their folders and their launcher.
pub const OLD_NAME: &str = "herdr-superwork";
pub fn env_now() -> Env { std::env::vars().collect() }
/// What is moved, from where to where. The old places followed the XDG_* variables, which is all those still do.
pub fn move_plan(home: &Path, env: &Env) -> Plan {
    let get = |k: &str| env.get(k).filter(|v| !v.is_empty()).cloned();
    let to = get("HERDR_PROJECTS_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(HOME_NAME));
    let old = |xdg: &str, rest: &[&str]| { let base = get(xdg).map(PathBuf::from).unwrap_or_else(|| rest.iter().fold(home.to_path_buf(), |p, r| p.join(r))); base.join(OLD_NAME) };
    let reports = PathBuf::from(format!("{}-reports", old("XDG_DATA_HOME", &[".local", "share"]).display()));
    Plan { file: to.join("moved.json"), aside: to.join("set-aside"), parts: vec![
        Part { what: "data", from: old("XDG_STATE_HOME", &[".local", "state"]), to: to.join("data"), key: "HERDR_PROJECTS_DATA_DIR" },
        Part { what: "settings", from: old("XDG_CONFIG_HOME", &[".config"]), to: to.join("settings"), key: "HERDR_PROJECTS_CONFIG_DIR" },
        Part { what: "reports", from: reports, to: to.join("reports"), key: "HERDR_PROJECTS_REPORTS_DIR" },
    ], home: to }
}
/// This copy of the program runs from a linked git worktree (a task's agent works on this very code): only the copy
/// the board runs from moves the board's files.
pub fn linked_worktree(bin: &Path) -> bool {
    let mut d = bin.to_path_buf();
    while d.pop() { if let Ok(m) = fs::metadata(d.join(".git")) { return m.is_file(); } }
    false
}
/// The command the old data folder's launcher names: `exec node '<path>' "$@"` (the old board) or `exec '<path>' "$@"`.
pub fn launcher_target(text: &str) -> Option<String> {
    for line in text.lines() {
        let rest = line.strip_prefix("exec node ").or_else(|| line.strip_prefix("exec "))?;
        let rest = rest.strip_suffix(" \"$@\"")?;
        let inner = rest.strip_prefix('\'')?.strip_suffix('\'')?;
        return Some(inner.replace("'\\''", "'"));
    }
    None
}
fn others_board(p: &Plan, bin: &Path, linked: bool) -> bool {
    if linked { return true; }
    let Some(text) = read_text(&p.parts[0].from.join("bin").join(OLD_NAME)) else { return false };
    let named = text.lines().find_map(|l| launcher_target(l)).unwrap_or_default();
    !named.is_empty() && Path::new(&named).exists() && real(Path::new(&named)) != real(bin) && !named.ends_with(".js")
}
fn free(file: &Path) -> PathBuf {
    if lstat(file).is_none() { return file.to_path_buf(); }
    (2..).map(|n| PathBuf::from(format!("{}.{n}", file.display()))).find(|f| lstat(f).is_none()).expect("a free name")
}
/// Takes a lock file for a moment. None when a running process holds it.
fn grab_now(file: &Path) -> Option<Option<fs::File>> {
    let Ok(f) = fs::File::open(file) else { return Some(None) };
    if crate::os::lock(&f, false) { Some(Some(f)) } else { None }
}
fn is_lock(file: &Path) -> bool { file.extension().map(|e| e == "lock").unwrap_or(false) && file.parent().and_then(|p| p.file_name()).map(|n| n == "locks").unwrap_or(false) }
fn keep(file: &Path, spot: &Path) { if fs::hard_link(file, spot).is_err() { let _ = copy_all(file, spot); } }
fn copy_all(from: &Path, to: &Path) -> std::io::Result<()> {
    if fs::symlink_metadata(from)?.is_dir() { fs::create_dir_all(to)?; for e in fs::read_dir(from)? { let e = e?; copy_all(&e.path(), &to.join(e.file_name()))?; } Ok(()) } else { fs::copy(from, to).map(|_| ()) }
}
fn mtime(m: &fs::Metadata) -> f64 { crate::os::mtime_ms(m) }
/// What the new place holds goes into the old one (the one in use); then the folder moves as one. Of a file both
/// have, the newer one stays in use and the other goes to the set-aside folder. Returns how many were set aside.
pub fn merge_into(from: &Path, to: &Path, aside: &Path) -> std::io::Result<usize> {
    let mut kept = 0;
    for e in fs::read_dir(from)? {
        let name = e?.file_name();
        let (a, b) = (from.join(&name), to.join(&name));
        let Some(sa) = lstat(&a) else { continue };
        let Some(sb) = lstat(&b) else { fs::rename(&a, &b)?; continue };
        if sa.is_dir() && sb.is_dir() { kept += merge_into(&a, &b, &aside.join(&name))?; fs::remove_dir(&a)?; continue; }
        if is_lock(&a) {
            match grab_now(&a) { None => return Err(std::io::Error::other(format!("{} is held in the new place", name.to_string_lossy()))), Some(_held) => { fs::remove_file(&a)?; } }
            continue;
        }
        fs::create_dir_all(aside)?;
        let spot = free(&aside.join(&name));
        if mtime(&sa) >= mtime(&sb) {
            keep(&b, &spot);
            if fs::rename(&a, &b).is_err() { let _ = fs::remove_dir_all(&b); let _ = fs::remove_file(&b); fs::rename(&a, &b)?; }
        } else if fs::rename(&a, &spot).is_err() { copy_all(&a, &spot)?; let _ = fs::remove_dir_all(&a); let _ = fs::remove_file(&a); }
        kept += 1;
    }
    Ok(kept)
}
/// Two paths change places in one step: there is no moment at which either is missing. False: not possible here.
#[cfg(windows)]
pub fn swap_now(_: &Path, _: &Path) -> bool { false }
#[cfg(unix)]
pub fn swap_now(a: &Path, b: &Path) -> bool {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let (Ok(ca), Ok(cb)) = (CString::new(a.as_os_str().as_bytes()), CString::new(b.as_os_str().as_bytes())) else { return false };
    #[cfg(target_os = "linux")]
    unsafe { libc::syscall(libc::SYS_renameat2, libc::AT_FDCWD, ca.as_ptr(), libc::AT_FDCWD, cb.as_ptr(), 2u32) == 0 }
    #[cfg(target_os = "macos")]
    unsafe { libc::renamex_np(ca.as_ptr(), cb.as_ptr(), libc::RENAME_SWAP) == 0 }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    { let _ = (ca, cb); false }
}
fn same_disk(a: &Path, b: &Path) -> bool { crate::os::same_disk(a, b) }
fn link(target: &Path, at: &Path) -> std::io::Result<()> { crate::os::symlink(target, at) }
/// One folder, in three steps each whole or not done (`<old>.moving` is the spare name the steps use):
///   1. two links are made: <old>.moving -> new, and new -> <old>.moving
///   2. <old> and <old>.moving change places: old -> new -> <old>.moving, which is now the folder
///   3. new and <old>.moving change places: new is the folder, old -> new; the spare link is removed
/// Returns what was done: "moved", "merged", "linked", or None.
pub fn move_one(p: &Part, rec: &mut Obj, file: &Path, aside: &Path, swap: &dyn Fn(&Path, &Path) -> bool) -> std::io::Result<Option<String>> {
    let tmp = PathBuf::from(format!("{}.moving", p.from.display()));
    let points = |at: &Path, to: &Path| fs::read_link(at).map(|x| x == to).unwrap_or(false);
    let save = |rec: &mut Obj, v: Obj| { rec.put(p.what, v); let _ = write_json(file, &Value::Object(rec.clone())); };
    let mut how: Option<String> = None;
    for _ in 0..6 {
        let (f, t, m) = (lstat(&p.from), lstat(&p.to), lstat(&tmp));
        let cur = rec.o(p.what).cloned().unwrap_or_default();
        if f.as_ref().map(|x| x.file_type().is_symlink()).unwrap_or(false) {
            if t.as_ref().map(|x| x.file_type().is_symlink()).unwrap_or(false) && m.as_ref().map(|x| x.is_dir()).unwrap_or(false) && points(&p.to, &tmp) {
                if !swap(&p.to, &tmp) { return Ok(how); }
                continue; // step 3
            }
            if !t.as_ref().map(|x| x.is_dir()).unwrap_or(false) || cur.s("state") != "moving" { return Ok(None); }
            if m.as_ref().map(|x| x.file_type().is_symlink()).unwrap_or(false) { fs::remove_file(&tmp)?; }
            let h = how.clone().or_else(|| cur.so("how").map(String::from)).unwrap_or_else(|| "moved".into());
            let mut done = cur.clone(); done.put("state", "done"); done.put("how", h.as_str()); done.put("at", now());
            save(rec, done);
            return Ok(Some(h));
        }
        if !f.as_ref().map(|x| x.is_dir()).unwrap_or(false) { return Ok(None); }
        let begun = t.as_ref().map(|x| x.file_type().is_symlink()).unwrap_or(false) && points(&p.to, &tmp);
        if !begun && real(&p.from) == real(&p.to) { return Ok(None); } // the new place is a link to this one already
        if t.as_ref().map(|x| !x.is_dir()).unwrap_or(false) && !begun { return Ok(None); } // something not ours sits there
        let mut moving = cur.clone();
        moving.put("from", p.from.display().to_string()); moving.put("to", p.to.display().to_string()); moving.put("state", "moving");
        if let Some(h) = &how { moving.put("how", h.as_str()); }
        save(rec, moving); // written first: a cut-off run is known for what it is
        if let Some(d) = p.to.parent() { fs::create_dir_all(d)?; }
        if t.as_ref().map(|x| x.is_dir() && !x.file_type().is_symlink()).unwrap_or(false) {
            let kept = merge_into(&p.to, &p.from, &aside.join(p.what))?;
            fs::remove_dir(&p.to)?;
            how = Some("merged".into());
            let mut r = rec.o(p.what).cloned().unwrap_or_default();
            r.put("how", "merged"); r.put("kept", r.i("kept") + kept as i64);
            rec.put(p.what, r);
            continue;
        }
        let stay = |rec: &mut Obj| -> std::io::Result<Option<String>> {
            for l in [&p.to, &tmp] { if lstat(l).map(|x| x.file_type().is_symlink()).unwrap_or(false) { let _ = fs::remove_file(l); } }
            link(&p.from, &p.to)?;
            let mut done = rec.o(p.what).cloned().unwrap_or_default(); done.put("state", "done"); done.put("how", "linked"); done.put("at", now());
            save(rec, done);
            Ok(Some("linked".into()))
        };
        if !same_disk(&p.from, p.to.parent().unwrap_or(Path::new("/"))) { return stay(rec); }
        if m.as_ref().map(|x| !x.file_type().is_symlink()).unwrap_or(false) { return Ok(None); } // the spare name is taken
        if m.is_none() { link(&p.to, &tmp)?; }
        if t.is_none() { link(&tmp, &p.to)?; }
        if !swap(&tmp, &p.from) { return stay(rec); } // step 2
        if how.is_none() { how = Some("moved".into()); }
        let mut r = rec.o(p.what).cloned().unwrap_or_default(); r.put("how", how.clone().unwrap()); rec.put(p.what, r);
    }
    Ok(None)
}
#[derive(Default)]
pub struct Moved { pub moved: Vec<(Part, String)>, pub line: String, pub stay: Vec<Part>, pub other: bool, pub busy: bool }
/// The move. Called at the start of every run of the program; almost always it finds nothing to do. `stay`: the
/// folders still in the old place when this returns: the run must work on those where they are.
pub fn move_home(home: &Path, env: &Env, bin: &Path, linked: Option<bool>, wait_ms: i64, swap: &dyn Fn(&Path, &Path) -> bool) -> Moved {
    let p = move_plan(home, env);
    let mut rec = read_obj(&p.file).unwrap_or_default();
    let get = |k: &str| env.get(k).filter(|v| !v.is_empty()).cloned();
    // A folder named outright by its own variable is left where it is, unless that names the old place itself.
    let mine = |x: &Part| match get(x.key) { None => true, Some(v) => { let v = absolute(Path::new(&v)); v == x.from || v == x.to } };
    let there = |x: &Part| lstat(&x.from).map(|m| m.is_dir()).unwrap_or(false) && fs::metadata(&x.from).is_ok() && real(&x.from) != real(&x.to);
    let due = |x: &Part, rec: &Obj| mine(x) && (there(x) || rec.at(x.what).s("state") == "moving");
    let out = |o: Moved| Moved { stay: p.parts.iter().filter(|x| mine(x) && there(x)).cloned().collect(), ..o };
    if !p.parts.iter().any(|x| due(x, &rec)) { return Moved::default(); }
    if others_board(&p, bin, linked.unwrap_or_else(|| linked_worktree(bin))) { return out(Moved { other: true, ..Default::default() }); }
    // One mover at a time. Another one at work: wait for it, then carry on with what it left.
    let lock = p.home.join("moving.lock");
    if fs::create_dir_all(&p.home).is_err() { return out(Moved::default()); }
    let until = now() + wait_ms;
    loop {
        match fs::OpenOptions::new().write(true).create_new(true).open(&lock) {
            Ok(mut f) => { let _ = std::io::Write::write_all(&mut f, serde_json::to_string(&marker()).unwrap_or_default().as_bytes()); break; }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if !marker_alive(&read_json(&lock).unwrap_or(Value::Null)) { let _ = fs::remove_file(&lock); continue; }
                if now() >= until { return out(Moved { busy: true, ..Default::default() }); }
                crate::clock::sleep_ms(20);
            }
            Err(_) => return out(Moved::default()),
        }
    }
    let mut moved: Vec<(Part, String)> = vec![];
    rec = read_obj(&p.file).unwrap_or_default();
    for x in p.parts.iter().filter(|x| due(x, &rec)).cloned().collect::<Vec<_>>() {
        if let Ok(Some(how)) = move_one(&x, &mut rec, &p.file, &p.aside, swap) { moved.push((x, how)); }
    }
    let _ = fs::remove_file(&lock);
    if moved.is_empty() { return out(Moved::default()); }
    let said: Vec<String> = moved.iter().map(|(x, how)| if how == "linked" { format!("{} stays in {} and is linked from there", x.what, short(&x.from, home)) } else { format!("{} (from {})", x.what, short(&x.from, home)) }).collect();
    let kept: i64 = rec.values().map(|v| v.i("kept")).sum();
    let line = format!("Moved the board's files into {}: {}.{}{}", short(&p.home, home), said.join(", "),
        if moved.iter().any(|(_, h)| h != "linked") { " The old places are links to the new ones, so whatever is running carries on." } else { "" },
        if kept > 0 { format!(" {kept} file{} in both places {} kept in {}.", if kept == 1 { " that was" } else { "s that were" }, if kept == 1 { "is" } else { "are" }, short(&p.aside, home)) } else { String::new() });
    add_line(&mut rec, &line);
    let _ = write_json(&p.file, &Value::Object(rec));
    out(Moved { moved, line, ..Default::default() })
}
/// One more line for the human, after what is still to be said.
pub fn add_line(rec: &mut Obj, line: &str) {
    let prev = if rec.at("told") == &json!(false) { rec.t("line") } else { String::new() };
    rec.put("line", [prev, line.to_string()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" "));
    rec.put("told", false);
}
/// The one line for the human, until it was shown once (the first run may be a hook, with nobody looking).
pub fn move_note(home: &Path, env: &Env) -> String {
    let r = read_obj(&move_plan(home, env).file).unwrap_or_default();
    if r.at("told") == &json!(false) { r.t("line") } else { String::new() }
}
pub fn move_told(home: &Path, env: &Env) {
    let f = move_plan(home, env).file;
    if let Some(mut r) = read_obj(&f) { if r.at("told") == &json!(false) { r.put("told", true); let _ = write_json(&f, &Value::Object(r)); } }
}
/// The path a folder inside a moved one had before the move, or None. A program started back then still knows its
/// folder by that path (the Coordinator's conversations are saved under it until its next restart).
pub fn moved_from(dir: &Path) -> Option<PathBuf> {
    let rec = read_obj(&move_plan(&user_home(), &env_now()).file).unwrap_or_default();
    for v in rec.values() {
        if v.s("state") != "done" || v.s("how") == "linked" || !v.b("to") || !v.b("from") { continue; }
        if let Ok(rest) = dir.strip_prefix(v.s("to")) { return Some(if rest.as_os_str().is_empty() { PathBuf::from(v.s("from")) } else { Path::new(v.s("from")).join(rest) }); }
    }
    None
}
/// Commands that are run with nobody reading their error output.
const QUIET: [&str; 8] = ["board", "hook", "turn-ended", "keeper", "keeper-work", "run", "serve", "remote"];
/// At the start of every run of the program: the move, and its one line said where a human reads it.
pub fn home_at_start(cmd: &str) {
    if std::env::var("HERDR_PROJECTS_INLINE").as_deref() == Ok("1") { return; }
    let (home, env) = (user_home(), env_now());
    let r = move_home(&home, &env, &bin(), None, 3000, &swap_now);
    // Not moved: this run works on those folders where they are, as it did before.
    for x in &r.stay { unsafe { std::env::set_var(x.key, &x.from); } }
    if QUIET.contains(&cmd) || !crate::os::is_tty(2) { return; }
    let line = move_note(&home, &env);
    if !line.is_empty() { eprintln!("{line}"); move_told(&home, &env); }
}
