//! Pinned machines and projects: this machine itself and its repos, each pinned or not, kept in `pins.json` in the
//! data folder (a task's pin is its own `pinned`). Another machine keeps its own and sends them with its tasks
//! (remote.rs: remote_list), as it sends its tasks' pins, so every board sees the same pins (docs/pinned.md).
use super::files::*;
use crate::err::Res;
use crate::json::*;
use std::path::PathBuf;

/// What is pinned of one machine: the machine itself, and its projects by their name on the board.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pins { pub machine: bool, pub repos: Vec<String> }
impl Pins {
    pub fn repo(&self, name: &str) -> bool { self.repos.iter().any(|r| r == name) }
    pub fn json(&self) -> Value { json!({ "machine": self.machine, "repos": self.repos }) }
    /// As a machine sent them (an older version sends none: nothing pinned).
    pub fn read(v: &Value) -> Pins { Pins { machine: v.b("machine"), repos: v.arr("repos").iter().map(text).filter(|r| !r.is_empty()).collect() } }
}
fn file() -> PathBuf { data_dir().join("pins.json") }
/// This machine's pins.
pub fn pins() -> Pins { Pins::read(&Value::Object(read_obj(&file()).unwrap_or_default())) }
/// Pin this machine (`repo` None) or one of its projects, or unpin it: what is pinned after, under the file's lock.
pub fn set(repo: Option<&str>, on: bool) -> Res<Pins> {
    with_lock_wait("pins", 5000, || {
        let mut p = pins();
        match repo {
            None => p.machine = on,
            Some(r) => { p.repos.retain(|x| x != r); if on { p.repos.push(r.to_string()); p.repos.sort(); } }
        }
        write_json(&file(), &p.json())?;
        Ok(p)
    })
}
