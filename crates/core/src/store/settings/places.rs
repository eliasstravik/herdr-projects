//! The places a setting can be said in, as they are on disk: your file (with your entry for each repo and each
//! coordinator's own entry in it), a task's own, and the repo's own committed file. Reading one checks every value: a mistake made by hand is a
//! problem with its setting's name, and that one setting is simply not said there.
use super::values::{check, nearest};
use crate::json::*;
use crate::store::files::*;
use crate::store::repos::git_out;
use crate::store::settings_list::{entry_of, names, repo_facts, Kind};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const REPO_FILE: &str = ".herdr-projects.json";
/// The name herdr-superwork gave that file: still read, in a repo that has only that one (docs/old-installs.md).
pub const OLD_REPO_FILE: &str = ".herdr-superwork.json";
pub fn config_file() -> PathBuf { config_dir().join("config.json") }

#[derive(Clone, Debug)]
pub struct Problem { pub place: String, pub name: String, pub why: String }

/// A file as written, nested ({ "review": { "rounds": 5 } }) or with dotted names, as one flat list of names and values.
fn leaves(o: &Value, prefix: &str, out: &mut Vec<(String, Value)>) {
    let Some(map) = o.as_object() else { return };
    for (k, v) in map {
        let name = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
        if v.is_object() && entry_of(&name).is_none() { leaves(v, &name, out); } else { out.push((name, v.clone())); }
    }
}
/// The settings said in one place, checked. `allowed`: the names this place may hold.
pub fn read(o: &Value, place: &str, allowed: &[&str], why: &str) -> (Obj, Vec<Problem>) {
    let mut values = Obj::new();
    let mut problems = vec![];
    let mut all = vec![];
    leaves(o, "", &mut all);
    for (name, v) in all {
        if !allowed.contains(&name.as_str()) {
            let near = nearest(&name, allowed);
            let why = if entry_of(&name).is_some() { format!("{name} {}", if why.is_empty() { "cannot be set here" } else { why }) }
                else { format!("unknown setting \"{name}\"{}", near.map(|n| format!(" (did you mean {n}?)")).unwrap_or_default()) };
            problems.push(Problem { place: place.into(), name, why });
            continue;
        }
        match check(&name, &v, "") { Ok(v) => { values.insert(name, v); } Err(e) => problems.push(Problem { place: place.into(), name, why: e.msg }) }
    }
    (values, problems)
}

// ---- you, and your entry for each repo: one file ----------------------------------------------------------------------
#[derive(Clone, Debug, Default)]
pub struct Mine { pub you: Obj, pub repos: BTreeMap<String, Obj>, pub coords: BTreeMap<String, Obj>, pub problems: Vec<Problem> }
static LAST: Mutex<Option<(String, Arc<Mine>)>> = Mutex::new(None);
/// Your settings. Asked for every setting that is looked up: the file is read again only when it changed on disk.
pub fn yours() -> Arc<Mine> {
    use std::os::unix::fs::MetadataExt;
    let f = config_file();
    let mark = std::fs::metadata(&f).map(|m| format!("{}|{}|{}|{}.{}|{}.{}", f.display(), m.ino(), m.size(), m.mtime(), m.mtime_nsec(), m.ctime(), m.ctime_nsec())).unwrap_or_default();
    if let Some((m, out)) = LAST.lock().unwrap().as_ref() { if !mark.is_empty() && *m == mark { return out.clone(); } }
    let out = Arc::new(read_yours());
    *LAST.lock().unwrap() = Some((mark, out.clone()));
    out
}
fn read_yours() -> Mine {
    let f = config_file();
    let (raw, broken) = match std::fs::read_to_string(&f) {
        Ok(t) => match serde_json::from_str::<Value>(&t) { Ok(v) => (v, false), Err(_) => (Value::Null, true) },
        Err(e) => (Value::Null, e.kind() != std::io::ErrorKind::NotFound),
    };
    let mut mine = raw.as_object().cloned().unwrap_or_default();
    let repos = mine.shift_remove("repos").unwrap_or(Value::Null);
    let coordinators = mine.shift_remove("coordinators").unwrap_or(Value::Null);
    let not_repo: Vec<&str> = names().into_iter().filter(|n| entry_of(n).map(|e| e.kind != Kind::Repo).unwrap_or(false)).collect();
    let (mut you, mut problems) = read(&Value::Object(mine), "your settings", &not_repo, "is one repo's own: say it under \"repos\", in that repo's entry");
    // A goal is one coordinator's own: said for you, it does not count.
    if you.shift_remove(super::coord::GOAL).is_some() { problems.push(Problem { place: "your settings".into(), name: super::coord::GOAL.into(), why: format!("{} is said for one coordinator only: under \"coordinators\", in its entry (or `set … --project-coordinator <repo>`)", super::coord::GOAL) }); }
    if broken { problems.push(Problem { place: "your settings".into(), name: String::new(), why: "it is not valid JSON, so nothing in it counts".into() }); }
    let not_board: Vec<&str> = names().into_iter().filter(|n| entry_of(n).map(|e| e.kind != Kind::Board && e.kind != Kind::Coord).unwrap_or(false)).collect();
    let mut entries = BTreeMap::new();
    if let Some(map) = repos.as_object() {
        for (repo, o) in map {
            let (v, p) = read(o, &format!("your entry for {repo}"), &not_board, "is a board setting, yours only: say it outside \"repos\"");
            entries.insert(repo.clone(), v);
            problems.extend(p);
        }
    }
    // Each coordinator's own entry: a coordinator's settings, and nothing else.
    let mut coords = BTreeMap::new();
    let own = crate::store::settings_list::coord();
    for (key, o) in super::coord::entries_of(&coordinators) {
        let (v, p) = read(&o, &format!("your settings for {}", super::coord::entry_words(&key)), &own, "is not a coordinator's setting: say it outside \"coordinators\"");
        coords.insert(key, v);
        problems.extend(p);
    }
    Mine { you, repos: entries, coords, problems }
}
/// Change one setting in your file: a value, or None to take it out. Everything else in the file stays as it is.
pub fn write_yours(name: &str, value: Option<&Value>, repo: &str) -> crate::err::Res<()> {
    write_at(name, value, &if repo.is_empty() { vec![] } else { vec!["repos".to_string(), repo.to_string()] })
}
/// The same at any place in your file: `[]` your own, `["repos", <repo>]`, `["coordinators", "main"]`. A place left
/// with nothing in it goes, and so does each part above it that it leaves empty.
pub fn write_at(name: &str, value: Option<&Value>, path: &[String]) -> crate::err::Res<()> {
    let mut file = read_obj(&config_file()).unwrap_or_default();
    let parts: Vec<&str> = name.split('.').collect();
    {
        let mut at: &mut Obj = &mut file;
        for p in path { at = at.obj_mut(p); }
        at.shift_remove(name); // the dotted spelling, where a hand wrote it that way
        fn drop(o: &mut Obj, parts: &[&str]) {
            if parts.len() == 1 { o.shift_remove(parts[0]); return; }
            if let Some(Value::Object(inner)) = o.get_mut(parts[0]) { drop(inner, &parts[1..]); if inner.is_empty() { o.shift_remove(parts[0]); } }
        }
        drop(at, &parts);
        if let Some(v) = value {
            let mut o = at;
            for p in &parts[..parts.len() - 1] { o = o.obj_mut(p); }
            o.insert(parts[parts.len() - 1].to_string(), v.clone());
        }
    }
    fn prune(o: &mut Obj, path: &[String]) {
        let Some((first, rest)) = path.split_first() else { return };
        if let Some(Value::Object(inner)) = o.get_mut(first) { prune(inner, rest); if inner.is_empty() { o.shift_remove(first); } }
    }
    prune(&mut file, path);
    write_json(&config_file(), &Value::Object(file))
}

// ---- the repo's own committed file: a few facts, no more ------------------------------------------------------------
/// Read from the commit a worktree starts from (`reference`), so an agent's own branch cannot change the rules it is
/// checked by; anything but the repo's facts is refused. Returns the facts, and one sentence per thing that did not count.
pub fn repo_file(root: &Path, reference: Option<&str>) -> (Obj, Vec<String>) {
    let text_of = |name: &str| match reference {
        Some(r) => git_out(root, &["show", &format!("{r}:{name}")]).unwrap_or_default(),
        None => read_text(&root.join(name)).unwrap_or_default(),
    };
    let mut text = text_of(REPO_FILE);
    if text.trim().is_empty() { text = text_of(OLD_REPO_FILE); }
    if text.trim().is_empty() { return (Obj::new(), vec![]); }
    let Ok(j) = serde_json::from_str::<Value>(&text) else { return (Obj::new(), vec![format!("{REPO_FILE} is not valid JSON: nothing in it counts")]) };
    let facts = repo_facts();
    let (values, problems) = read(&j, REPO_FILE, &facts, &format!("is refused: the repo's file may only hold {}", facts.join(", ")));
    (values, problems.into_iter().map(|p| format!("{REPO_FILE}: {}", p.why)).collect())
}
