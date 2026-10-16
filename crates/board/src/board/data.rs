//! What the board reads, read away from the keyboard: by the loader, once a second and after anything you did, on a
//! thread of its own. The board's state takes it in as one event, so a key press never waits for a file, herdr or git.
//! A task the board looks closer at (the selected one, the one the chat talks to) has a `snap` of what only a closer
//! look gives: its settings and where each comes from, its conversation, its screen, its plan: the same `snap`
//! another machine sends with its tasks (remote.rs), so both are drawn alike.
use crate::agents::screen_text::screen_text;
use crate::coordinator::look::{look, screen, Look};
use crate::flow::{live::board, talk::task_talk, task::is_agent_task};
use crate::herdr::cli::{current_socket, session_name};
use crate::json::*;
use crate::shown::states::as_row;
use crate::store::{automations::automations, inbox::{clear_dispatch, dispatches}, keys::{repo_keys, Keys}, marks::tilde, repos::{branch_of, can_worktree, folder_of, known_repos, repo_of}, settings::{lookup::{in_repo, setting, Scope}, show::{problems, rows}}};
use std::collections::BTreeMap;

/// Another machine, as the board knows it: its tasks as rows, its automations, its Coordinator as it sent it
/// (coordinator/far.rs) and its machine's and repos' coordinators (`coords`, by level), and why it did not answer; `sidebar`: its herdr list of machines as it last sent it
/// (remote/sidebar.rs), None from one that sends none.
#[derive(Clone, Debug, Default)]
pub struct Remote { pub machine: crate::remote::Machine, pub items: Vec<crate::board::state::Rec>, pub autos: Vec<Obj>, pub ok: bool, pub busy: bool, pub fail_at: i64, pub why: Option<Value>, pub version: String, pub coord: Value, pub coords: Value, pub sidebar: Option<Vec<crate::remote::sidebar::Entry>> }

#[derive(Clone, Debug, Default)]
pub struct Data {
    /// This machine's tasks, archived ones too, as rows.
    pub items: Vec<crate::board::state::Rec>,
    /// This machine's automations, each with what its row and panel show (`shown`, as another machine sends it).
    pub autos: Vec<Obj>,
    /// Words on their way to another machine's Coordinator.
    pub jobs: Vec<Obj>,
    pub coord: Look, pub coord_screen: String, pub can_dispatch: bool,
    /// The machine's or a repo's coordinator of this machine the chat talks to: its level ("machine",
    /// "project:<repo>"), how it stands, and its screen (update/coords.rs).
    pub sub: Option<(String, Look, String)>,
    pub keys: Keys, pub repos: Vec<String>, pub roots: BTreeMap<String, String>, pub session: String, pub notify: String,
    /// The repo the board is in (where it was opened, or where a draft was typed), its folder, and the tasks' file.
    pub repo: String, pub repo_root: String,
    /// This machine's name, for its heading on the board of every machine.
    pub host: String,
    /// herdr's approve action asked this board to ask about a task; a line the board key left for this board.
    pub approve: String, pub heard: Option<(String, String)>,
    pub loaded: bool,
    /// Which read this is (`reads_started`), counted from the program's start.
    pub seq: u64,
}
/// What the board wants a closer look at.
#[derive(Clone, Debug, Default)]
pub struct Wants { pub ids: Vec<String>, pub cwd: String, pub own_pane: String, pub web: bool, pub coord: String }

/// The repo the board is in, worked out once for each folder: a busy lock keeps what was known.
pub fn repo_here(cwd: &str) -> (String, String) {
    static HERE: std::sync::Mutex<Option<(String, (String, String))>> = std::sync::Mutex::new(None);
    let mut h = HERE.lock().unwrap();
    if let Some((c, r)) = h.as_ref() { if c == cwd && !r.0.is_empty() { return r.clone(); } }
    let r = repo_of(std::path::Path::new(cwd)).map(|(n, p)| (n, p.display().to_string())).unwrap_or_else(|_| h.as_ref().map(|x| x.1.clone()).unwrap_or_default());
    *h = Some((cwd.to_string(), r.clone()));
    r
}
static READS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// How many reads of the tasks have started: a read numbered higher than this began after now, so it sees what was
/// just done.
pub fn reads_started() -> u64 { READS.load(std::sync::atomic::Ordering::SeqCst) }
/// One read of everything. `wants`: the tasks to look at closer.
pub fn load(wants: &Wants) -> Data {
    let seq = READS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    let socket = current_socket();
    let mut items: Vec<Obj> = board(true).iter().map(as_row).collect();
    for r in items.iter_mut() { if wants.ids.iter().any(|w| w == r.s("id")) { let s = snap(r); r.put("snap", s); } }
    let items = items.into_iter().map(std::sync::Arc::new).collect();
    let now = crate::clock::now();
    let autos: Vec<Obj> = automations().into_iter().map(|a| shown_auto(a, now)).collect();
    let jobs: Vec<Obj> = dispatches();
    // A finished instruction is forgotten without a word: its row goes, and what it did is on the board.
    for j in &jobs { if j.s("state") == "done" { clear_dispatch(j.s("id")); } }
    let jobs = jobs.into_iter().filter(|j| j.s("state") != "done").rev().collect();
    let coord = look(&socket);
    let coord_screen = if !coord.readable && coord.status.is_some() { screen_text(&screen(&coord, &socket)) } else { String::new() };
    let sub = (!wants.coord.is_empty()).then(|| { let co = crate::coordinator::co::Co::of(&socket, &wants.coord); let v = look(&co); let sc = if !v.readable && v.status.is_some() { screen_text(&screen(&v, &co)) } else { String::new() }; (wants.coord.clone(), v, sc) });
    let here = repo_here(&wants.cwd);
    let roots: BTreeMap<String, String> = known_repos().iter().map(|(k, v)| (k.clone(), text(v))).collect();
    Data {
        items, autos, jobs, coord, coord_screen, sub, can_dispatch: crate::coordinator::who::ready(&socket), keys: repo_keys(), repos: roots.keys().cloned().collect(), roots,
        session: session_name(&socket), notify: setting("board.notify", None).as_str().unwrap_or("").to_string(),
        repo: here.0, repo_root: here.1, host: crate::remote::check::hostname(),
        // herdr's approve action and a line left by the board key are for a board in a terminal, never the web board.
        approve: if wants.web { String::new() } else { crate::actions::finish_line::take_approve(&socket, &wants.own_pane) }, heard: if wants.web { None } else { crate::board::run::say::heard(&wants.own_pane, &socket) }, loaded: true, seq,
    }
}
/// An automation with what its row and its panel show, read here: its firings, newest first, its own settings, every
/// work setting its next task will have, whether its tasks are research only.
pub fn shown_auto(a: Obj, now: i64) -> Obj {
    use crate::automations::{list as l, show::{firings, row_of}};
    let rows = l::setting_rows(&a);
    let mut o = a.clone();
    let fired = firings(&a);
    let oldest_first: Vec<Value> = fired.iter().rev().cloned().collect();
    o.put("research", l::research_only(&a));
    o.put("shown", json!({ "firings": fired.into_iter().take(20).collect::<Vec<_>>(), "own": rows.iter().filter(|x| x.own).map(|x| json!({ "name": x.name, "value": x.value, "own": true })).collect::<Vec<_>>(),
        "settings": rows.iter().map(|x| x.json()).collect::<Vec<_>>() }));
    o.put("rowNow", serde_json::to_value(row_of(&a, now, Some(&oldest_first))).unwrap_or(Value::Null));
    o
}
/// A closer look at one task of this machine.
pub fn snap(t: &Obj) -> Value {
    let folder = folder_of(t.s("id"));
    let tree = if t.b("worktree") && std::path::Path::new(t.s("worktree")).exists() { t.t("worktree") } else { String::new() };
    let live = is_agent_task(t) && !t.b("archivedAt");
    let talk = if live { task_talk(t, 12, 20000) } else { Value::Null };
    // A program whose conversation cannot be read: the words on its screen instead.
    let screen = if live && !talk.b("readable") { crate::board::screens::screen_of(t) } else { String::new() };
    let plan = if t.at("plan").b("at") { crate::plan::read_plan(t) } else { String::new() };
    let at = if tree.is_empty() { folder.clone() } else { tree.clone().into() };
    json!({ "talk": talk, "screen": screen, "plan": plan, "landsOn": if t.s("stage") == "approval" { json!(crate::sync::lands_on(t)) } else { Value::Null },
        "folder": tilde(&folder.display().to_string()), "root": folder.display().to_string(), "tree": tilde(&tree), "treePath": tree, "branch": branch_of(&at),
        "settings": rows(&Scope::task(t)).iter().map(|r| r.json()).collect::<Vec<_>>(), "problems": problems(Some(t)), "canWorktree": can_worktree(&folder), "repo": in_repo(t),
        "worktreeGone": t.b("worktree") && tree.is_empty() })
}
