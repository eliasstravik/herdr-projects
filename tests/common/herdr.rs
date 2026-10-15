//! A stand-in for herdr, for tests: no server, no terminals, no model calls. Its world is one JSON record (spaces,
//! tabs, panes, their agents), answered for in the test's own thread (herdr::cli::set_fake), so a test's hundreds of
//! calls start no program. Agents are scripted (agents.rs): each prompt plays the next step of the script for its role.
use serde_json::{json, Value};
use std::cell::RefCell;
use sw::herdr::cli::{Fake, Ran};
use sw::json::*;

pub struct FakeHerdr { pub st: RefCell<Value>, pub dir: std::path::PathBuf, pub script: RefCell<Value>, pub env: Vec<(String, String)> }
pub struct Exit(pub i32, pub String, pub String);
pub type Out = Result<Exit, Exit>;
pub fn ok(result: Value) -> Out { Err(Exit(0, format!("{}\n", json!({ "id": "fake", "result": result })), String::new())) }
pub fn fail(code: &str, message: &str) -> Out { Err(Exit(1, String::new(), format!("{}\n", json!({ "id": "fake", "error": { "code": code, "message": message } })))) }
pub fn out(text: &str) -> Out { Err(Exit(0, text.into(), String::new())) }
pub fn empty() -> Value { json!({ "seq": 0, "nextW": 1, "nextP": 100, "workspaces": [], "panes": [], "tabs": [], "notifications": [], "metadata": [], "focus": [], "calls": [], "dialogs": {} }) }
pub fn arr<'a>(v: &'a mut Value, k: &str) -> &'a mut Vec<Value> { if !v.at(k).is_array() { v[k] = json!([]); } v[k].as_array_mut().unwrap() }

impl Fake for FakeHerdr {
    fn run(&self, args: &[String], _input: &str) -> Ran {
        let r = self.answer(args);
        let Exit(status, stdout, stderr) = match r { Ok(e) | Err(e) => e };
        Ran { status, stdout, stderr }
    }
    fn kids(&self, pid: i64) -> Vec<i64> { if pid == 1 { vec![2] } else { vec![] } }
    /// Ending a fake agent's process: its pane is at its shell again.
    fn kill(&self, pid: i64) { for p in self.st.borrow_mut()["panes"].as_array_mut().unwrap() { if p.at("agent").i("pid") == pid { p["agent"] = Value::Null; } } }
}
impl FakeHerdr {
    pub fn ws_out(st: &Value, w: &Value) -> Value {
        let mut o = json!({ "workspace_id": w.at("workspace_id"), "label": w.at("label"), "pane_count": st.arr("panes").iter().filter(|p| p.at("workspace_id") == w.at("workspace_id")).count() });
        if w.at("worktree").is_object() { o["worktree"] = w.at("worktree").clone(); }
        o
    }
    pub fn tab_out(st: &Value, t: &Value) -> Value { json!({ "tab_id": t.at("tab_id"), "workspace_id": t.at("workspace_id"), "label": t.at("label"), "pane_count": st.arr("panes").iter().filter(|p| p.at("tab_id") == t.at("tab_id")).count(), "focused": t.s("tab_id") == format!("{}:t1", t.s("workspace_id")) }) }
    pub fn pane_out(p: &Value) -> Value {
        let mut o = json!({ "pane_id": p.at("pane_id"), "workspace_id": p.at("workspace_id"), "tab_id": p.at("tab_id"), "terminal_id": p.at("terminal_id"), "cwd": p.at("cwd"), "foreground_cwd": p.at("cwd"),
            "agent": if p.at("agent").is_object() { p.at("agent").at("kind").clone() } else { Value::Null }, "agent_status": if p.at("agent").is_object() { p.at("agent").at("status").clone() } else { json!("unknown") } });
        if p.b("display_agent") { o["display_agent"] = p.at("display_agent").clone(); }
        o
    }
    pub fn agent_out(p: &Value) -> Value {
        let a = p.at("agent");
        let mut o = json!({ "agent": a.at("kind"), "agent_status": a.at("status"), "name": a.at("name"), "pane_id": p.at("pane_id"), "workspace_id": p.at("workspace_id"), "tab_id": p.at("tab_id"), "terminal_id": p.at("terminal_id"), "cwd": p.at("cwd"), "foreground_cwd": p.at("cwd"), "state_change_seq": a.at("seq") });
        if p.b("display_agent") { o["display_agent"] = p.at("display_agent").clone(); }
        if !a.at("completion").is_null() { o["completion_seq"] = a.at("completion").clone(); }
        if a.b("session") { o["agent_session"] = json!({ "kind": "id", "value": a.at("session") }); }
        o
    }
    pub fn new_tab(st: &mut Value, w: usize, id: &str, label: &str) -> Value {
        let n: i64 = id.split(":t").nth(1).and_then(|x| x.parse().ok()).unwrap_or(1);
        let next = st["workspaces"][w].i("nextT").max(1).max(n + 1);
        st["workspaces"][w]["nextT"] = json!(next);
        let t = json!({ "tab_id": id, "workspace_id": st["workspaces"][w].at("workspace_id"), "label": label });
        arr(st, "tabs").push(t.clone());
        t
    }
    pub fn new_workspace(st: &mut Value, cwd: &str, label: Option<&str>, worktree: Value) -> Value {
        let n = st.i("nextW"); st["nextW"] = json!(n + 1);
        let id = format!("w{n}");
        let home = sw::store::files::user_home().display().to_string();
        let label = label.map(String::from).unwrap_or_else(|| if cwd == home { "~".into() } else { std::path::Path::new(cwd).file_name().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default() });
        let mut w = json!({ "workspace_id": id, "label": label, "cwd": cwd });
        if worktree.is_object() { w["worktree"] = worktree; }
        arr(st, "workspaces").push(w);
        let np = st.i("nextP"); st["nextP"] = json!(np + 1);
        let p = json!({ "pane_id": format!("{id}:p1"), "workspace_id": id, "tab_id": format!("{id}:t1"), "terminal_id": format!("term_{np}"), "cwd": cwd, "agent": Value::Null });
        arr(st, "panes").push(p.clone());
        let wi = st.arr("workspaces").len() - 1;
        let t = Self::new_tab(st, wi, &format!("{id}:t1"), "1");
        let w = st["workspaces"][wi].clone();
        json!({ "workspace": Self::ws_out(st, &w), "tab": Self::tab_out(st, &t), "root_pane": Self::pane_out(&p) })
    }
    pub fn drop_empty(st: &mut Value) {
        let panes = st.arr("panes").to_vec();
        arr(st, "tabs").retain(|t| panes.iter().any(|p| p.at("tab_id") == t.at("tab_id")));
        arr(st, "workspaces").retain(|w| panes.iter().any(|p| p.at("workspace_id") == w.at("workspace_id")));
    }
    pub fn bump(st: &mut Value, pi: usize, status: &str, complete: bool) {
        let seq = st.i("seq") + 1; st["seq"] = json!(seq);
        let a = &mut st["panes"][pi]["agent"];
        a["status"] = json!(status); a["seq"] = json!(seq);
        if complete { a["completion"] = json!(seq); }
    }
    /// Closing notes of turns that have ended, written now (agents.rs): stamped after the board armed the turn they end,
    /// as a real agent's note always is, however slow the machine running the test.
    pub fn notes(&self) {
        let notes: Vec<Value> = std::mem::take(arr(&mut self.st.borrow_mut(), "pendingNotes"));
        for n in notes {
            let p = std::path::PathBuf::from(n[0].as_str().unwrap_or(""));
            if let Some(d) = p.parent() { let _ = std::fs::create_dir_all(d); }
            let _ = std::fs::write(&p, n[1].as_str().unwrap_or(""));
            let armed = sw::store::tasks::get(n[2].as_str().unwrap_or("")).map(|t| t.i("armedAt")).unwrap_or(0);
            let at = std::time::UNIX_EPOCH + std::time::Duration::from_millis((armed + 2).max(sw::clock::now()) as u64);
            if let Ok(fh) = std::fs::File::options().write(true).open(&p) { let _ = fh.set_modified(at); }
        }
    }
    fn answer(&self, argv: &[String]) -> Out {
        let mut st = self.st.borrow_mut();
        // Long enough for a call's last flags also under macOS's long temp folders (/private/var/folders/…).
        let calls = arr(&mut st, "calls"); calls.push(json!(argv.join(" ").chars().take(2000).collect::<String>()));
        let flag = |n: &str| argv.iter().position(|a| a == n).and_then(|i| argv.get(i + 1)).cloned();
        let has = |n: &str| argv.iter().any(|a| a == n);
        let env_of = || -> Value { let mut o = serde_json::Map::new(); for (i, a) in argv.iter().enumerate() { if a == "--env" { if let Some((k, v)) = argv.get(i + 1).and_then(|x| x.split_once('=')) { o.insert(k.into(), json!(v)); } } } Value::Object(o) };
        let (area, verb) = (argv.first().map(|s| s.as_str()).unwrap_or(""), argv.get(1).map(|s| s.as_str()).unwrap_or(""));
        let arg2 = argv.get(2).cloned().unwrap_or_default();
        let find_pane = |st: &Value, id: &str| st.arr("panes").iter().position(|p| p.s("pane_id") == id);
        match (area, verb) {
            ("config", "check") => return out("config: ok\n"),
            ("server", "reload-config") => return ok(json!({ "type": "config_reload", "status": "applied", "diagnostics": [] })),
            ("workspace", "list") => { let ws: Vec<Value> = st.arr("workspaces").iter().map(|w| Self::ws_out(&st, w)).collect(); return ok(json!({ "type": "workspace_list", "workspaces": ws })); }
            ("workspace", "create") => {
                let cwd = flag("--cwd").unwrap_or_else(|| ".".into());
                let made = Self::new_workspace(&mut st, &cwd, flag("--label").as_deref(), Value::Null);
                if let Some(pi) = find_pane(&st, made.at("root_pane").s("pane_id")) { st["panes"][pi]["env"] = env_of(); }
                let mut m = made.as_object().cloned().unwrap(); m.insert("type".into(), json!("workspace_created"));
                return ok(Value::Object(m));
            }
            ("workspace", _) => {
                let Some(wi) = st.arr("workspaces").iter().position(|w| w.s("workspace_id") == arg2) else { return fail("workspace_not_found", &format!("workspace {arg2} not found")) };
                let w = st["workspaces"][wi].clone();
                match verb {
                    "get" => return ok(json!({ "type": "workspace_info", "workspace": Self::ws_out(&st, &w) })),
                    "focus" => { arr(&mut st, "focus").push(json!(format!("workspace {arg2}"))); return ok(json!({ "type": "workspace_info", "workspace": Self::ws_out(&st, &w) })); }
                    "rename" => { st["workspaces"][wi]["label"] = json!(argv[3..].join(" ")); let w = st["workspaces"][wi].clone(); return ok(json!({ "type": "workspace_info", "workspace": Self::ws_out(&st, &w) })); }
                    "report-metadata" => { arr(&mut st, "metadata").push(json!({ "workspace": arg2, "args": argv[3..] })); return ok(json!({ "type": "ok" })); }
                    "close" => { arr(&mut st, "workspaces").remove(wi); arr(&mut st, "panes").retain(|p| p.s("workspace_id") != arg2); Self::drop_empty(&mut st); return ok(json!({ "type": "ok" })); }
                    _ => {}
                }
            }
            ("worktree", _) => return super::worktrees::worktree(self, &mut st, verb, &flag, has("--force")),
            ("tab", _) => return self.tab(&mut st, verb, &arg2, argv, &flag, &env_of),
            ("pane", _) => return super::panes::pane(&mut st, verb, &arg2, argv, &flag, &env_of),
            ("agent", _) => { drop(st); return super::agents::agent(self, verb, &arg2, argv, &flag); }
            ("notification", _) => { arr(&mut st, "notifications").push(json!({ "title": arg2, "body": flag("--body").unwrap_or_default() })); return ok(json!({ "type": "notification_show", "shown": true })); }
            ("plugin", "pane") => {
                arr(&mut st, "focus").push(json!("board"));
                let placement = flag("--placement").unwrap_or_else(|| "popup".into());
                st["boardPlacement"] = json!(placement);
                if placement != "tab" || st.arr("workspaces").is_empty() { return ok(json!({ "type": "ok" })); }
                let wid = st["workspaces"][0].t("workspace_id");
                let n = st["workspaces"][0].i("nextT").max(2);
                let t = Self::new_tab(&mut st, 0, &format!("{wid}:t{n}"), &n.to_string());
                let np = st.i("nextP"); st["nextP"] = json!(np + 1);
                let q = json!({ "pane_id": format!("{wid}:board"), "workspace_id": wid, "tab_id": t.at("tab_id"), "terminal_id": format!("term_{np}"), "cwd": st["workspaces"][0].at("cwd"), "agent": Value::Null, "label": "Work board" });
                arr(&mut st, "panes").push(q.clone());
                return ok(json!({ "type": "plugin_pane_opened", "plugin_pane": { "entrypoint": "board", "plugin_id": "herdr-projects", "pane": Self::pane_out(&q) } }));
            }
            ("plugin", "list") => return ok(json!({ "type": "plugin_list", "plugins": [] })),
            ("integration", "status") => return out(&st.s("integrations").to_string()), // what a test says of herdr's integrations
            ("machine", _) => return out(&format!("{}\n", std::fs::read_to_string(self.dir.join("machines.json")).unwrap_or_else(|_| "[]".into()).trim())),
            _ => {}
        }
        fail("unknown_command", &format!("fake herdr does not know: {}", argv.join(" ")))
    }
    fn tab(&self, st: &mut Value, verb: &str, arg2: &str, argv: &[String], flag: &dyn Fn(&str) -> Option<String>, env_of: &dyn Fn() -> Value) -> Out {
        match verb {
            "focus" => { arr(st, "focus").push(json!(format!("tab {arg2}"))); return ok(json!({ "type": "ok" })); }
            "list" => { let w = flag("--workspace"); let ts: Vec<Value> = st.arr("tabs").iter().filter(|t| w.as_deref().map(|w| t.s("workspace_id") == w).unwrap_or(true)).map(|t| Self::tab_out(st, t)).collect(); return ok(json!({ "type": "tab_list", "tabs": ts })); }
            "create" => {
                let wid = flag("--workspace").unwrap_or_default();
                let Some(wi) = st.arr("workspaces").iter().position(|w| w.s("workspace_id") == wid) else { return fail("workspace_not_found", &format!("workspace {wid} not found")) };
                if st.b("tabFails") { return fail("timeout", "timed out"); }
                let n = st["workspaces"][wi].i("nextT").max(2);
                let t = Self::new_tab(st, wi, &format!("{wid}:t{n}"), &flag("--label").unwrap_or_else(|| n.to_string()));
                let count = st.arr("panes").iter().filter(|p| p.s("workspace_id") == wid).count() + 1;
                let mut id = format!("{wid}:p{count}");
                while st.arr("panes").iter().any(|p| p.s("pane_id") == id) { id.push('x'); }
                let np = st.i("nextP"); st["nextP"] = json!(np + 1);
                let q = json!({ "pane_id": id, "workspace_id": wid, "tab_id": t.at("tab_id"), "terminal_id": format!("term_{np}"), "cwd": flag("--cwd").unwrap_or_else(|| st["workspaces"][wi].t("cwd")), "env": env_of(), "agent": Value::Null });
                arr(st, "panes").push(q.clone());
                return ok(json!({ "type": "tab_created", "tab": Self::tab_out(st, &t), "root_pane": Self::pane_out(&q) }));
            }
            _ => {}
        }
        let Some(ti) = st.arr("tabs").iter().position(|t| t.s("tab_id") == arg2) else { return fail("tab_not_found", &format!("tab {arg2} not found")) };
        match verb {
            "get" => { let t = st["tabs"][ti].clone(); ok(json!({ "type": "tab_info", "tab": Self::tab_out(st, &t) })) }
            "rename" => { st["tabs"][ti]["label"] = json!(argv[3..].join(" ")); let t = st["tabs"][ti].clone(); ok(json!({ "type": "tab_info", "tab": Self::tab_out(st, &t) })) }
            "close" => { arr(st, "panes").retain(|p| p.s("tab_id") != arg2); Self::drop_empty(st); ok(json!({ "type": "ok" })) }
            _ => fail("unknown_command", "tab"),
        }
    }
}
