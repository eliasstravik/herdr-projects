//! The skills the text box offers, read for the loader (data.rs): of the program whoever the chat talks to runs, in
//! the folder it runs in (update/skills.rs names them: a task's agent, this machine's Coordinator or one of its
//! heading's coordinators). Read again at most every few seconds while the list is wanted.
use crate::agents::skills::{offered, Skill};
use crate::coordinator::co::Co;
use crate::coordinator::look::Look;
use crate::json::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

/// How long a list read is used again.
const FRESH_MS: i64 = 5000;
static READ: LazyLock<Mutex<HashMap<(String, PathBuf), (i64, Vec<Skill>)>>> = LazyLock::new(Default::default);

/// The program and its folder: a task's agent ("task\t<id>"), or a coordinator of this machine ("coord\t<level>",
/// `top` and `sub` as the loader just read them).
fn whose(spec: &str, socket: &str, top: &Look, sub: Option<&(String, Look, String)>) -> Option<(String, PathBuf)> {
    let (what, of) = spec.split_once('\t')?;
    match what {
        "task" => { let t = crate::store::tasks::get(of)?; Some((t.t("agent"), crate::store::repos::cwd_of(&t))) }
        "coord" => {
            let look = if of.is_empty() { Some(top) } else { sub.filter(|x| x.0 == of).map(|x| &x.1) }?;
            let kind = look.run.as_ref().map(|r| r.t("kind")).unwrap_or_default();
            Some((kind, crate::coordinator::state::home(Co::of(socket, of))))
        }
        _ => None,
    }
}
/// The list for `spec` (update/skills.rs: `wanted`), with the spec it is for (empty where that one is gone); None for
/// nobody.
pub fn read(spec: &str, socket: &str, top: &Look, sub: Option<&(String, Look, String)>) -> Option<(String, Vec<Skill>)> {
    if spec.is_empty() { return None; }
    let Some((kind, dir)) = whose(spec, socket, top, sub) else { return Some((spec.to_string(), vec![])) };
    let now = crate::clock::now();
    let key = (kind.clone(), dir.clone());
    if let Some((at, list)) = READ.lock().unwrap().get(&key) { if now - at < FRESH_MS && now >= *at { return Some((spec.to_string(), list.clone())); } }
    let list = offered(&kind, &dir);
    READ.lock().unwrap().insert(key, (now, list.clone()));
    Some((spec.to_string(), list))
}
