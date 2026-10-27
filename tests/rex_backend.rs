//! The Rex backend through a stand-in for the rex program: a socket names its backend, a space is a window of its
//! repo's session, a worktree is git's with a window in it, an agent is typed at a block's shell and its turns are
//! what the program reports (OSC 7501), and a prompt is pasted and entered on its own.
use crate::common;
use common::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;
use sw::herdr::backend::{for_socket, socket_for};
use sw::herdr::cli::Ran;
use sw::herdr::rex::run::{set_fake, RexFake};
use sw::herdr::Herdr;
use sw::json::*;

/// Sessions, windows and blocks as Rex holds them, and every key and text sent.
#[derive(Default)]
struct FakeRex { st: RefCell<Value>, n: RefCell<i64> }
fn out(v: Value) -> Ran { Ran { status: 0, stdout: v.to_string(), stderr: String::new() } }
fn bad(code: &str) -> Ran { Ran { status: 1, stdout: String::new(), stderr: format!("Error: no (status=404 code={code})") } }
impl FakeRex {
    fn id(&self, kind: &str) -> String { *self.n.borrow_mut() += 1; format!("{kind}:{}", self.n.borrow()) }
    fn block(&self, cwd: &str) -> Value { json!({ "block_id": self.id("block"), "label": "shell", "cwd": cwd, "fg": "zsh", "typed": "", "records": [] }) }
    fn window(&self, label: &str, cwd: &str) -> Value { json!({ "window_id": self.id("window"), "label": label, "blocks": [self.block(cwd)] }) }
    fn blocks_mut<'a>(st: &'a mut Value) -> impl Iterator<Item = &'a mut Value> { st["sessions"].as_array_mut().into_iter().flatten().flat_map(|s| s["windows"].as_array_mut().into_iter().flatten()).flat_map(|w| w["blocks"].as_array_mut().into_iter().flatten()) }
    fn with_block<T>(&self, id: &str, f: impl FnOnce(&mut Value) -> T) -> Option<T> { let mut st = self.st.borrow_mut(); Self::blocks_mut(&mut st).find(|b| b.s("block_id") == id).map(f) }
    fn report(&self, block: &str, state: &str, at: &str) { self.with_block(block, |b| b["records"] = json!([{ "state": state, "app": "claude", "updated_at": at, "owner": { "pid": 0 } }])); }
    fn sent(&self) -> Vec<Value> { self.st.borrow().arr("sent").to_vec() }
}
impl RexFake for FakeRex {
    fn run(&self, a: &[String], input: &str) -> Ran {
        assert_eq!(a.first().map(String::as_str), Some("--autostart=false"), "never starts a server: {a:?}");
        let a = &a[1..];
        let w = |i: usize| a.get(i).map(String::as_str).unwrap_or("");
        let flag = |f: &str| a.iter().position(|x| x == f).and_then(|i| a.get(i + 1)).cloned().unwrap_or_default();
        let mut st = self.st.borrow_mut();
        if !st["sessions"].is_array() { st["sessions"] = json!([]); st["sent"] = json!([]); }
        match (w(0), w(1)) {
            ("ls", _) => out(json!({ "sessions": st.arr("sessions").iter().map(|s| json!({ "session_id": s.at("session_id"), "label": s.at("label") })).collect::<Vec<_>>() })),
            ("new", _) => {
                let win = self.window(&flag("--window"), &flag("-c"));
                let s = json!({ "session_id": self.id("session"), "label": w(1), "windows": [win] });
                st["sessions"].as_array_mut().unwrap().push(s.clone());
                out(json!({ "session_id": s.at("session_id"), "initial_windows": [{ "window_id": win.at("window_id"), "block_ids": [win["blocks"][0].at("block_id")] }] }))
            }
            ("window", "new") => {
                let win = self.window(w(4), &flag("-c"));
                let Some(s) = st["sessions"].as_array_mut().unwrap().iter_mut().find(|s| s.s("session_id") == flag("-s")) else { return bad("session_not_found") };
                s["windows"].as_array_mut().unwrap().push(win.clone());
                out(json!({ "window_id": win.at("window_id"), "block_ids": [win["blocks"][0].at("block_id")] }))
            }
            ("window", "close") => { for s in st["sessions"].as_array_mut().unwrap() { s["windows"].as_array_mut().unwrap().retain(|x| !a[2..].contains(&x.t("window_id"))); } out(json!({})) }
            ("window", "rename") | ("window", "focus") | ("block", "focus") | ("block", "zoom") => { st["sent"].as_array_mut().unwrap().push(json!({ "do": format!("{} {}", w(0), w(1)), "id": a.get(2) })); out(json!({})) }
            ("split", _) => {
                let b = self.block(&flag("-c"));
                let anchor = flag("-b");
                for s in st["sessions"].as_array_mut().unwrap() { for win in s["windows"].as_array_mut().unwrap() { if win.arr("blocks").iter().any(|x| x.s("block_id") == anchor) { win["blocks"].as_array_mut().unwrap().push(b.clone()); } } }
                out(json!({ "block_ids": [b.at("block_id")] }))
            }
            ("block", "rename") => { drop(st); self.with_block(w(2), |b| b["label"] = json!(w(3))).map(|_| out(json!({}))).unwrap_or_else(|| bad("bad_request")) }
            ("block", "call") => {
                drop(st);
                let method = w(4);
                self.with_block(w(3), |b| out(match method {
                    "process" => json!({ "child": { "pid": 1, "name": "login" }, "ancestors": [{ "pid": 1, "name": "login" }], "foreground": { "pid": 2, "name": b.at("fg"), "cwd": b.at("cwd") } }),
                    "program_status" => json!({ "records": b.at("records") }),
                    "size" => json!({ "columns": 80, "rows": 3 }),
                    _ => json!({}),
                })).unwrap_or_else(|| bad("bad_request"))
            }
            ("api", "call") => {
                let s = st.arr("sessions").iter().find(|s| s.s("session_id") == flag("-s")).cloned();
                let Some(s) = s else { return bad("session_not_found") };
                out(json!({ "session_id": s.at("session_id"), "windows": s.arr("windows").iter().map(|w| json!({ "window_id": w.at("window_id"), "label": w.at("label"), "layers": [{ "blocks": w.arr("blocks").iter().map(|b| json!({ "block_id": b.at("block_id"), "label": b.at("label") })).collect::<Vec<_>>() }] })).collect::<Vec<_>>() }))
            }
            ("send", _) => { st["sent"].as_array_mut().unwrap().push(json!({ "block": flag("-b"), "text": input })); drop(st); self.with_block(&flag("-b"), |b| { let t = b.t("typed") + input; b["typed"] = json!(t); }); out(json!({})) }
            ("send-key", _) => {
                st["sent"].as_array_mut().unwrap().push(json!({ "block": flag("-b"), "keys": a[3..] })); drop(st);
                // Enter at the shell starts what was typed: its first word is the program.
                self.with_block(&flag("-b"), |b| if a[3..].iter().any(|k| k == "enter") { if b.s("fg") == "zsh" { let p = b.t("typed").split_whitespace().next().unwrap_or("zsh").to_string(); b["fg"] = json!(p); } b["typed"] = json!(""); });
                out(json!({}))
            }
            ("capture", _) => { drop(st); self.with_block(&flag("-b"), |_| Ran { status: 0, stdout: "one\ntwo\nthree\nfour\n".into(), stderr: String::new() }).unwrap_or_else(|| bad("bad_request")) }
            ("block", "close") => { for s in st["sessions"].as_array_mut().unwrap() { for win in s["windows"].as_array_mut().unwrap() { win["blocks"].as_array_mut().unwrap().retain(|b| !a[2..].contains(&b.t("block_id"))); } } out(json!({})) }
            _ => bad("unknown"),
        }
    }
}
fn rex(_w: &World) -> (Rc<FakeRex>, Herdr) {
    let f = Rc::new(FakeRex::default());
    set_fake(Some(f.clone()));
    (f, Herdr::new(Some("rex:".into())))
}

#[test]
fn a_socket_names_its_backend_and_the_setting_picks_rex_for_new_work_only() {
    assert_eq!(for_socket(Some("/tmp/herdr.sock")).name(), "herdr");
    assert_eq!(for_socket(None).name(), "herdr");
    assert_eq!(for_socket(Some("rex:")).name(), "rex");
    assert_eq!(socket_for("herdr", "/s/herdr.sock"), "/s/herdr.sock");
    assert_eq!(socket_for("rex", "/s/herdr.sock"), "rex:");
    assert_eq!(socket_for("rex", "rex:box"), "rex:box", "a board on a Rex server keeps its own");
    assert_eq!(sw::store::settings_list::entry_of("workspace.terminal").map(|e| e.builtin.clone()), Some(json!("herdr")));
}

#[test]
fn a_space_is_a_window_of_its_repos_session_with_the_boards_socket_in_its_shell() {
    let w = world();
    let (f, h) = rex(&w);
    let root = w.root.display().to_string();
    let made = h.workspace_create(&root, "Task 1", &[("A".into(), "b".into())]).unwrap();
    let ws = made.at("workspace").t("workspace_id");
    let pane = made.at("root_pane").t("pane_id");
    assert_eq!(made.at("tab").t("tab_id"), ws, "the first tab is the window the space was made with");
    let tab = h.tab_create(&ws, "", "Reviewer", &[]).unwrap();
    assert_eq!(h.tabs(&ws).unwrap().len(), 2);
    assert_eq!(h.workspaces().unwrap().len(), 1);
    let second = h.workspace_create(&root, "Task 2", &[]).unwrap();
    assert_eq!(f.st.borrow().arr("sessions").len(), 1, "one session per repo: {}", second);
    let split = h.pane_split(&pane, "", "right", &[]).unwrap();
    let panes = h.panes(Some(&ws)).unwrap();
    assert_eq!(panes.len(), 3, "{panes:?}");
    assert!(panes.iter().all(|p| p.s("workspace_id") == ws && p.s("cwd") == root));
    assert_eq!(h.pane_get(split.s("pane_id")).map(|p| p.t("tab_id")), Some(ws.clone()));
    assert_eq!(h.pane_read(&pane, "recent", Some(2)).unwrap(), "three\nfour");
    h.workspace_close(&ws).unwrap();
    assert!(h.tab_get(tab.at("tab").s("tab_id")).is_none());
    assert_eq!(h.workspaces().unwrap().len(), 1);
    set_fake(None);
}

#[test]
fn the_board_opens_in_its_own_window_started_once_and_two_repos_of_one_name_are_two_sessions() {
    let w = world();
    let (f, h) = rex(&w);
    let _cwd = Cwd::set(&w.root);
    let b = h.backend();
    let first = b.open_board().unwrap().unwrap();
    assert_eq!(b.open_board().unwrap(), Some(first.clone()), "found again, not made again");
    let starts = f.sent().iter().filter(|s| s.s("text").ends_with(" board\n")).count();
    assert_eq!(starts, 1, "{:?}", f.sent());
    assert!(f.sent().iter().any(|s| s.s("do") == "window focus"), "the board is shown");
    let other = w.base.join("elsewhere").join(w.root.file_name().unwrap());
    std::fs::create_dir_all(&other).unwrap();
    common::git(&other, &["init", "-q"]);
    h.workspace_create(&other.display().to_string(), "", &[]).unwrap();
    assert_eq!(f.st.borrow().arr("sessions").len(), 2);
    h.pane_focus(&first).unwrap();
    assert_eq!(f.sent().last().unwrap().s("do"), "block focus");
    set_fake(None);
}

/// The test's working folder, put back when it ends.
struct Cwd(std::path::PathBuf);
impl Cwd { fn set(p: &std::path::Path) -> Self { let old = std::env::current_dir().unwrap(); std::env::set_current_dir(p).unwrap(); Cwd(old) } }
impl Drop for Cwd { fn drop(&mut self) { let _ = std::env::set_current_dir(&self.0); } }

#[test]
fn a_worktree_is_gits_with_a_window_in_it_and_goes_with_its_space() {
    let w = world();
    let (_f, h) = rex(&w);
    let made = h.worktree_create(&w.root.display().to_string(), "work/1-x", "HEAD", "Task").unwrap();
    let path = made.at("worktree").t("path");
    assert!(std::path::Path::new(&path).join(".git").exists(), "{made}");
    assert!(made.at("workspace").at("worktree").b("is_linked_worktree"));
    let ws = made.at("workspace").t("workspace_id");
    assert_eq!(h.worktree_open("", &path, "").unwrap().b("already_open"), true);
    std::fs::write(std::path::Path::new(&path).join("new.txt"), "x").unwrap();
    assert_eq!(h.worktree_remove(&ws, false).unwrap_err().code, "worktree_remove_failed");
    assert!(h.workspace_get(&ws).is_some(), "a worktree with changes keeps its space");
    h.worktree_remove(&ws, true).unwrap();
    assert!(!std::path::Path::new(&path).exists());
    assert!(h.workspace_get(&ws).is_none());
    set_fake(None);
}

#[test]
fn an_agent_is_typed_at_the_shell_its_turns_are_what_it_reports_and_a_prompt_is_pasted_then_entered() {
    let w = world();
    let (f, h) = rex(&w);
    let pane = h.workspace_create(&w.root.display().to_string(), "T", &[]).unwrap().at("root_pane").t("pane_id");
    let b = h.backend();
    let a = b.agent_start("worker 1", "claude", &pane, &["--model".into(), "x y".into()], 5000).unwrap();
    assert_eq!(a.s("agent"), "claude");
    assert!(f.sent().iter().any(|s| s.s("text") == "claude --model 'x y'"), "{:?}", f.sent());
    f.with_block(&pane, |b| b["records"] = json!([{ "state": "done", "updated_at": "0", "owner": { "pid": 99 } }]));
    assert_eq!(h.agent_get("worker 1").unwrap().map(|a| a.t("agent_status")), Some("unknown".into()), "an earlier program's done is not this one's");
    f.report(&pane, "working", "1");
    let working = h.agent_get(&pane).unwrap().unwrap();
    assert_eq!(working.s("agent_status"), "working");
    assert!(working.at("completion_seq").is_null(), "no finished turn yet");
    f.report(&pane, "idle", "2");
    let done = h.agent_get(&pane).unwrap().unwrap();
    assert_eq!((done.s("agent_status"), done.i("completion_seq")), ("idle", 1));
    assert!(done.i("state_change_seq") > working.i("state_change_seq"));
    assert_eq!(h.agent_get(&pane).unwrap().unwrap().i("state_change_seq"), done.i("state_change_seq"), "the same reading again is no change");
    b.agent_prompt(&pane, "hello\nthere").unwrap();
    let sent = f.sent();
    let n = sent.len();
    assert_eq!(sent[n - 2].s("text"), "\u{1b}[200~hello\nthere\u{1b}[201~");
    assert_eq!(sent[n - 1].at("keys"), &json!(["enter"]));
    f.report(&pane, "blocked", "3");
    assert_eq!(b.agent_prompt(&pane, "x").unwrap_err().code, "agent_blocked");
    b.agent_keys(&pane, &["esc"]).unwrap();
    assert_eq!(f.sent().last().unwrap().at("keys"), &json!(["escape"]));
    assert_eq!(h.agents().unwrap().len(), 1);
    set_fake(None);
}
