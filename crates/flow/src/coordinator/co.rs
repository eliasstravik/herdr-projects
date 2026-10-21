//! Which coordinator: the top-level Coordinator of a herdr session, the machine coordinator of this machine, or the
//! project coordinator of one repo. Each is a conversation of its own, kept in a state file of its own, in a tab named
//! after it. Written as one string (`key`): the socket alone for the top-level one, so every place that names the
//! Coordinator by its herdr session still does; the others add their level after a separator no path holds.
use crate::herdr::cli::session_name;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Level { #[default] Top, Machine, Project(String) }
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Co { pub socket: String, pub level: Level }
const SEP: char = '\u{1f}';
pub const AGENT_NAME_MAX: usize = 32;

impl Co {
    pub fn top(socket: &str) -> Co { Co { socket: socket.into(), level: Level::Top } }
    pub fn machine(socket: &str) -> Co { Co { socket: socket.into(), level: Level::Machine } }
    pub fn project(socket: &str, repo: &str) -> Co { Co { socket: socket.into(), level: Level::Project(repo.into()) } }
    /// From its level in words, as another machine is asked for it: "" the top-level one, "machine", "project:<repo>".
    pub fn of(socket: &str, to: &str) -> Co {
        match to { "" => Co::top(socket), "machine" => Co::machine(socket), t => Co::project(socket, t.strip_prefix("project:").unwrap_or(t)) }
    }
    /// Its level in words (`of` reads them back).
    pub fn to(&self) -> String { match &self.level { Level::Top => String::new(), Level::Machine => "machine".into(), Level::Project(r) => format!("project:{r}") } }
    pub fn key(&self) -> String { match &self.level { Level::Top => self.socket.clone(), Level::Machine => format!("{}{SEP}machine", self.socket), Level::Project(r) => format!("{}{SEP}project{SEP}{r}", self.socket) } }
    pub fn is_top(&self) -> bool { self.level == Level::Top }
    /// Its own entry in config.json's `coordinators` (store/settings/coord.rs): "main", "machine", "project:<repo>".
    pub fn settings_key(&self) -> String { match &self.level { Level::Top => "main".into(), Level::Machine => "machine".into(), Level::Project(r) => format!("project:{r}") } }
    pub fn repo(&self) -> Option<&str> { match &self.level { Level::Project(r) => Some(r), _ => None } }
    /// What the human reads, on the board, its tab and herdr's sidebar: "Coordinator", "vm-str coordinator",
    /// "herdr-projects coordinator" ("~ coordinator" for the home folder's).
    pub fn name(&self) -> String {
        match &self.level { Level::Top => crate::status_words::COORDINATOR.into(), Level::Machine => named(&crate::remote::check::hostname()), Level::Project(r) => named(&crate::shown::words::repo_label(r)) }
    }
    /// Its agent's inner name in herdr: one each, so a machine runs them side by side. herdr takes lowercase letters,
    /// digits, `-` and `_`, at most 32 characters: room is left for the `-2` that tells two alike apart (pump.rs).
    pub fn agent_name(&self) -> String {
        match &self.level {
            Level::Top => "sw-coordinator".into(), Level::Machine => "sw-coordinator-machine".into(),
            Level::Project(r) => {
                let tail: String = r.to_lowercase().chars().map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' { c } else { '-' }).collect();
                format!("sw-coordinator-{tail}").chars().take(AGENT_NAME_MAX - 3).collect::<String>().trim_end_matches('-').to_string()
            }
        }
    }
    /// The end of its lock's and its marks' names: nothing for the top-level one, which keeps the names it always had.
    pub fn suffix(&self) -> String { match &self.level { Level::Top => String::new(), Level::Machine => "-machine".into(), Level::Project(r) => format!("-project-{}", safe(r)) } }
    pub fn mark(&self, what: &str) -> String { format!("{what}-{}{}", session_name(&self.socket), self.suffix()) }
    /// The repo's folder, for a project coordinator: where it works.
    pub fn root(&self) -> Option<PathBuf> { self.repo().and_then(|r| crate::store::repos::known_repos().get(r).map(|v| PathBuf::from(crate::json::text(v)))) }
}
/// "vm-str coordinator": a coordinator named after what it coordinates.
pub fn named(what: &str) -> String { format!("{what} coordinator") }
fn safe(r: &str) -> String { r.chars().map(|c| if c.is_ascii_alphanumeric() || "_.+-".contains(c) { c } else { '-' }).collect::<String>().trim_start_matches('.').to_string() }

impl From<&str> for Co {
    fn from(k: &str) -> Co {
        let mut p = k.split(SEP);
        let socket = p.next().unwrap_or("");
        match (p.next(), p.next()) { (Some("machine"), _) => Co::machine(socket), (Some("project"), Some(r)) => Co::project(socket, r), _ => Co::top(socket) }
    }
}
impl From<&String> for Co { fn from(k: &String) -> Co { Co::from(k.as_str()) } }
impl From<String> for Co { fn from(k: String) -> Co { Co::from(k.as_str()) } }
impl From<&Co> for Co { fn from(c: &Co) -> Co { c.clone() } }
