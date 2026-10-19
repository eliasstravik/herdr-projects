//! The fake herdr's agents: started in a pane, prompted, sent keys, read. Each prompt plays the next step of the
//! script for its role (worker, reviewer for a name ending in -r, picker for -pick, the Coordinator): it writes files,
//! commits, writes the closing note, the plan, the verdict or the comparison where the board tells an agent to, and
//! may end its turn with a recorded ending (tests/fixtures/endings) and run the agent's own turn-end report. A step's
//! `then` is a step of the same turn played later: on the board's `after`-th look at the agent (3; 0: never), or by
//! `go_on`.
use super::herdr::{arr, fail, ok, out, FakeHerdr as F, Out};
use super::worktrees::git;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use sw::json::*;

pub fn agent(f: &F, verb: &str, arg2: &str, argv: &[String], flag: &dyn Fn(&str) -> Option<String>) -> Out {
    let mut st = f.st.borrow_mut();
    match verb {
        "list" => { let a: Vec<Value> = st.arr("panes").iter().filter(|p| p.at("agent").is_object()).map(F::agent_out).collect(); return ok(json!({ "type": "agent_list", "agents": a })); }
        "start" => {
            let Some(pi) = flag("--pane").and_then(|id| st.arr("panes").iter().position(|p| p.s("pane_id") == id)) else { return fail("pane_not_found", "pane not found") };
            // A pane where an agent runs, and `noShell`: that many starts find the pane's shell still starting. herdr sees
            // no shell in it either way and types nothing (herdr 0.9.5).
            let busy = format!("agent target pane {} is not an available shell", st["panes"][pi].s("pane_id"));
            if st["panes"][pi].at("agent").is_object() { return fail("agent_pane_busy", &busy); }
            if st.i("noShell") > 0 { st["noShell"] = json!(st.i("noShell") - 1); return fail("agent_pane_busy", &busy); }
            let kind = flag("--kind").unwrap_or_default();
            let rest: Vec<Value> = argv.iter().position(|a| a == "--").map(|i| argv[i + 1..].iter().map(|x| json!(x)).collect()).unwrap_or_default();
            // `refuses`: per kind, the arguments its installed version does not know. It quits at once, as real
            // herdr shows it: the pane back at its shell and `timed out waiting for agent startup` at the timeout.
            if let Some(bad) = rest.iter().map(text).find(|x| st.at("refuses").arr(&kind).iter().any(|r| text(r) == *x)) {
                let (cwd, cmd) = (st["panes"][pi].t("cwd"), std::iter::once(kind.clone()).chain(rest.iter().map(text)).collect::<Vec<_>>().join(" "));
                st["panes"][pi]["screen"] = json!(format!("user@host:{cwd}$ {cmd}\nerror: unexpected argument '{bad}' found\n\nUsage: {kind} [OPTIONS] [PROMPT]\nuser@host:{cwd}$ \n"));
                return fail("timeout", "timed out waiting for agent startup");
            }
            // `slowShell`: the pane's shell is still starting, and its terminal takes at most 4096 bytes on one line. A
            // longer start command arrives cut off, the shell waits at its continuation prompt and herdr times out.
            // `missing`: the kinds that are not installed: the shell says so, and herdr times out too.
            let typed = sw::agents::hand_limits::typed_command(&kind, &rest.iter().map(text).collect::<Vec<_>>());
            let cwd = st["panes"][pi].t("cwd");
            if st.b("slowShell") && typed.len() > 4096 {
                st["panes"][pi]["screen"] = json!(format!("user@host:{cwd}$ {}\n>", typed.chars().scan(0, |n, c| { *n += c.len_utf8(); (*n <= 4096).then_some(c) }).collect::<String>()));
                return fail("timeout", "timed out waiting for agent startup");
            }
            if st.at("missing").as_array().map(|m| m.iter().any(|k| text(k) == kind)).unwrap_or(false) {
                st["panes"][pi]["screen"] = json!(format!("user@host:{cwd}$ {typed}\nbash: {kind}: command not found\nuser@host:{cwd}$ "));
                return fail("timeout", "timed out waiting for agent startup");
            }
            // `canonical`: that many starts find the pane's shell not at its prompt yet, its terminal taking 1024 bytes
            // on a line (macOS): herdr types nothing and says so. `canonicalLate`: it says so, and the program came up.
            // `canonicalName`: only the agents whose name ends so.
            let refused = st.i("canonical") > 0 && arg2.ends_with(st.s("canonicalName"));
            if refused { st["canonical"] = json!(st.i("canonical") - 1); }
            let canonical = format!("input contains a {}-byte line but the terminal is in canonical mode with a 1024-byte limit; wait for the shell prompt and retry", typed.len().max(1025));
            if refused && !st.b("canonicalLate") { return fail("invalid_input", &canonical); }
            let seq = st.i("seq") + 1; st["seq"] = json!(seq);
            let mut a = json!({ "kind": kind, "name": arg2, "status": "idle", "seq": seq, "args": rest, "pid": 5000000 + seq });
            if st.i("slowStart") > 0 { a["boot"] = json!(st.i("slowStart")); }
            st["panes"][pi]["agent"] = a;
            if refused { return fail("invalid_input", &canonical); }
            let d = st.at("dialogs").i(&kind);
            if d > 0 { st["dialogs"][&kind] = json!(d - 1); st["panes"][pi]["agent"]["status"] = json!("blocked"); return fail("agent_not_ready", &format!("agent {arg2} is blocked during startup and is not ready for prompts")); }
            let p = st["panes"][pi].clone();
            return ok(json!({ "type": "agent_started", "agent": F::agent_out(&p) }));
        }
        _ => {}
    }
    let Some(pi) = st.arr("panes").iter().position(|p| p.at("agent").is_object() && (p.s("pane_id") == arg2 || p.at("agent").s("name") == arg2)) else { return fail("agent_not_found", &format!("agent target {arg2} not found")) };
    match verb {
        "get" => {
            if st.b("agentGetFails") { return fail("timeout", "timed out"); }
            let due = { let a = &mut st["panes"][pi]["agent"]; a.at("then").is_object() && a.i("thenIn") > 0 && { let n = a.i("thenIn") - 1; a["thenIn"] = json!(n); n == 0 } };
            if due { drop(st); go_on(f, pi); st = f.st.borrow_mut(); }
            let p = st["panes"][pi].clone(); ok(json!({ "type": "agent_info", "agent": F::agent_out(&p) }))
        }
        "prompt" => {
            if st["panes"][pi]["agent"].s("status") == "blocked" { return fail("agent_blocked", "agent is blocked"); }
            let typed = read_named(&argv.get(3).cloned().unwrap_or_default());
            let box_ = st["panes"][pi]["agent"].at("box").clone();
            let text = if box_.is_string() { format!("{}\n{typed}", text_of(&box_)) } else { typed };
            if st["panes"][pi]["agent"].i("boot") > 0 || st.i("swallow") > 0 {
                if st["panes"][pi]["agent"].i("boot") <= 0 { st["swallow"] = json!(st.i("swallow") - 1); }
                st["panes"][pi]["agent"]["box"] = json!(text);
                let p = st["panes"][pi].clone();
                return ok(json!({ "type": "agent_prompted", "agent": F::agent_out(&p) }));
            }
            st["panes"][pi]["agent"]["box"] = Value::Null;
            drop(st);
            play(f, pi, &text);
            let st = f.st.borrow();
            let p = st["panes"].get(pi).cloned().unwrap_or(Value::Null);
            ok(json!({ "type": "agent_prompted", "agent": if p.at("agent").is_object() { F::agent_out(&p) } else { Value::Null } }))
        }
        "send-keys" => {
            let keys: Vec<String> = argv[3..].to_vec();
            let a = &mut st["panes"][pi]["agent"];
            arr(a, "keys").extend(keys.iter().map(|k| json!(k)));
            let enter = keys.iter().any(|k| k == "enter");
            if a.at("box").is_string() && enter {
                if st.i("deaf") > 0 { st["deaf"] = json!(st.i("deaf") - 1); return ok(json!({ "type": "ok" })); }
                let t = text_of(st["panes"][pi]["agent"].at("box")); st["panes"][pi]["agent"]["box"] = Value::Null;
                drop(st); play(f, pi, &t); return ok(json!({ "type": "ok" }));
            }
            if st["panes"][pi]["agent"].s("status") == "blocked" && enter { F::bump(&mut st, pi, "idle", true); }
            else if keys.iter().any(|k| k == "esc") && st["panes"][pi]["agent"].s("status") == "working" { F::bump(&mut st, pi, "idle", true); }
            ok(json!({ "type": "ok" }))
        }
        "focus" => { let pid = st["panes"][pi].t("pane_id"); arr(&mut st, "focus").push(json!(format!("agent {pid}"))); let p = st["panes"][pi].clone(); ok(json!({ "type": "agent_info", "agent": F::agent_out(&p) })) }
        "read" => out(st["panes"][pi]["agent"].s("screen")),
        _ => fail("unknown_command", "agent"),
    }
}
/// A message handed over in a file (too long to type in): the agent reads that file, and takes it as the message.
fn read_named(typed: &str) -> String {
    let re = regex::Regex::new(r"^\[This message is too long to be typed in, so it is in the file (\S+)\. ").unwrap();
    re.captures(typed).and_then(|c| std::fs::read_to_string(&c[1]).ok()).unwrap_or_else(|| typed.to_string())
}
fn text_of(v: &Value) -> String { v.as_str().unwrap_or("").to_string() }
fn data() -> PathBuf { sw::store::files::data_dir() }
/// The task the agent was told it is on (its first prompt's task line), and its folder on the board.
fn told(first: &str) -> Option<String> { regex::Regex::new(r"^Work board task (\S+),").unwrap().captures(first).map(|c| c[1].to_string()) }
fn task_dir(id: &str) -> PathBuf { let mut p = data().join("tasks"); for part in id.split('/') { p.push(part); } p }
fn write(path: &Path, text: &str) { if let Some(d) = path.parent() { let _ = std::fs::create_dir_all(d); } let _ = std::fs::write(path, text); }

/// The rest of the agent's turn (its step's `then`), now.
pub fn go_on(f: &F, pi: usize) {
    let (step, prompt) = { let mut st = f.st.borrow_mut(); let a = &mut st["panes"][pi]["agent"]; (a["then"].take(), a.arr("prompts").last().map(text).unwrap_or_default()) };
    if step.is_object() { run(f, pi, &prompt, step); }
}
/// One scripted turn.
pub fn play(f: &F, pi: usize, prompt: &str) {
    let name = f.st.borrow()["panes"][pi]["agent"].t("name");
    let coordinator = name.starts_with("sw-coordinator");
    let role = if coordinator { "coordinator" } else if regex::Regex::new(r"-pick(-\d+)?$").unwrap().is_match(&name) { "picker" } else if regex::Regex::new(r"-r(-\d+)?$").unwrap().is_match(&name) { "reviewer" } else { "worker" };
    let step = { let mut sc = f.script.borrow_mut(); let q = arr(&mut sc, role); if q.is_empty() { Value::Null } else { q.remove(0) } };
    let step = if !step.is_null() { step } else { match role { "picker" => json!({ "comparison": { "pick": "A", "why": "A does all of it with the smallest change." } }), "reviewer" => json!({ "verdict": { "meets": true, "findings": [], "text": "Looks right. Nothing found." } }), "coordinator" => json!({}), _ => json!({ "note": "Done." }) } };
    {
        let mut st = f.st.borrow_mut();
        arr(&mut st["panes"][pi]["agent"], "prompts").push(json!(prompt));
        F::bump(&mut st, pi, "working", false);
    }
    run(f, pi, prompt, step);
}
fn run(f: &F, pi: usize, prompt: &str, step: Value) {
    let (name, cwd) = { let st = f.st.borrow(); (st["panes"][pi]["agent"].t("name"), st["panes"][pi].t("cwd")) };
    let coordinator = name.starts_with("sw-coordinator");
    let first = f.st.borrow()["panes"][pi]["agent"].arr("prompts").first().map(text).unwrap_or_default();
    let dir = Path::new(&cwd);
    if !coordinator {
        if step.b("merge") { let _ = git(dir, &["merge", "-q", "--no-edit", step.s("merge")]); }
        for (rel, body) in step.at("files").as_object().cloned().unwrap_or_default() { write(&dir.join(&rel), &text(&body)); }
        if step.b("commit") { let _ = git(dir, &["add", "-A"]); let _ = git(dir, &["-c", "user.email=agent@test", "-c", "user.name=agent", "commit", "-qm", step.s("commit")]); }
        // What the agent does with git itself after that (pushes to main, merges its own pull request): each a git call.
        for c in step.arr("git") { let a: Vec<String> = c.as_array().map(|a| a.iter().map(text).collect()).unwrap_or_default(); let a: Vec<&str> = a.iter().map(|s| s.as_str()).collect(); if let Err(e) = git(dir, &a) { panic!("git {a:?}: {e}"); } }
        let mine = told(&first).filter(|id| task_dir(id).join("task.json").exists()).map(|id| task_dir(&id));
        // The note is written once its turn has really ended: between the board's jobs, after the one that armed the turn
        // (herdr.rs: notes), as a real agent writes it well after the prompt went in.
        if step.at("note").is_string() { let file = match &mine { Some(m) if !step.b("oldNote") => m.join("closing-note.md"), _ => dir.join(".herdr-projects").join(format!("closing-{}.md", regex::Regex::new(r"-(\d+[a-e]?)(?:-[0-9a-f]{4})?(?:-r(?:-\d+)?|-pick(?:-\d+)?)?$").unwrap().captures(&name).map(|c| c[1].to_string()).unwrap_or_default())) }; arr(&mut f.st.borrow_mut(), "pendingNotes").push(json!([file.display().to_string(), step.s("note"), told(&first).unwrap_or_default()])); }
        if let (true, Some(m)) = (step.b("plan"), &mine) { write(&m.join("artifacts").join("plan").join("plan.md"), step.s("plan")); }
        if !step.at("comparison").is_null() { let id = told(prompt).unwrap_or_else(|| "x/0".into()); let c = step.at("comparison"); write(&task_dir(&id).join("compare").join("comparison.json"), &if c.is_string() { text(c) } else { c.to_string() }); }
        if let (true, Some(id)) = (step.b("report"), told(&first)) { let mut p = sw::store::files::reports_dir(); for part in id.split('/') { p.push(part); } write(&p.join("report.md"), step.s("report")); }
        if step.at("verdict").is_object() {
            if let Some(m) = &mine {
                for (n, body) in step.at("evidence").as_object().cloned().unwrap_or_default() { write(&m.join("review").join(n), &text(&body)); }
                let mut v = step.at("verdict").clone();
                if !v.b("commit") || v.s("commit") == "HEAD" { v["commit"] = json!(git(dir, &["rev-parse", "HEAD"]).unwrap_or_default()); }
                write(&m.join("review").join("verdict.json"), &v.to_string());
            }
        }
    } else if step.at("verdict").is_object() && step.b("reviewTask") {
        let id = step.s("reviewTask");
        let task = sw::store::tasks::get(id).unwrap();
        let mut verdict = step.at("verdict").clone();
        verdict["commit"] = json!(git(&sw::store::repos::cwd_of(&task), &["rev-parse", "HEAD"]).unwrap_or_default());
        write(&task_dir(id).join("review/verdict.json"), &verdict.to_string());
    } else if step.b("said") {
        let st = f.st.borrow();
        let args: Vec<String> = st["panes"][pi]["agent"].arr("args").iter().map(text).collect();
        if let (Some(at), Some(home)) = (args.iter().position(|a| a == "--session-id" || a == "--resume"), std::env::var_os("CLAUDE_CONFIG_DIR")) {
            let real = std::fs::canonicalize(dir).unwrap_or(dir.to_path_buf()).display().to_string();
            let file = Path::new(&home).join("projects").join(real.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>()).join(format!("{}.jsonl", args[at + 1]));
            let seq = st.i("seq");
            let now = sw::clock::iso(sw::clock::now());
            let lines = format!("{}\n{}\n", json!({ "type": "user", "message": { "role": "user", "content": prompt }, "timestamp": now }), json!({ "type": "assistant", "uuid": format!("u{seq}"), "message": { "id": format!("msg_{seq}"), "role": "assistant", "content": [{ "type": "text", "text": step.s("said") }], "stop_reason": "end_turn" }, "timestamp": now }));
            if let Some(d) = file.parent() { let _ = std::fs::create_dir_all(d); }
            use std::io::Write; let _ = std::fs::OpenOptions::new().create(true).append(true).open(&file).and_then(|mut fh| fh.write_all(lines.as_bytes()));
        }
        drop(st);
    }
    if step.b("helper") { super::endings::helper_ends(f, pi, step.s("helper") == "running"); }
    if step.b("ends") || step.b("hook") { super::endings::end_turn(f, pi, prompt, &step); }
    let mut st = f.st.borrow_mut();
    if step.b("session") { st["panes"][pi]["agent"]["session"] = step.at("session").clone(); }
    if step.b("screen") { st["panes"][pi]["agent"]["screen"] = step.at("screen").clone(); }
    if step.b("exit") { st["panes"][pi]["agent"] = Value::Null; return; }
    if step.at("then").is_object() { let a = &mut st["panes"][pi]["agent"]; a["then"] = step.at("then").clone(); a["thenIn"] = json!(if step.has("after") { step.i("after") } else { 3 }); }
    let status = if step.b("status") { step.t("status") } else { "done".into() };
    if status != "working" { F::bump(&mut st, pi, &status, status == "done" || status == "idle"); }
}
