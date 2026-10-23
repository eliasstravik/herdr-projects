//! A clean-up put off because its space is in use says what keeps it: a turn in its agent's terminal, an agent herdr
//! does not answer for, a process beside the agent that is not its helper (by name, and why), or another of its panes.
//! A helper wired to the agent keeps nothing; `details` lists every thing.
use crate::common;
use common::*;
use serde_json::{json, Value};
use sw::json::*;

/// What a clean-up ends before the worktree goes (a dev server handed to init), and what it names for you to end.
mod procs;

fn kept(t: &Obj) -> String { t.arr("kept").first().map(text).unwrap_or_default() }
fn pane(w: &World, id: &str, f: impl FnOnce(&mut Value)) { w.patch(|s| { let p = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.s("pane_id") == id).unwrap(); f(p); }); }
/// A task approved and merged once `before` did something to its space: the task as it then is.
fn merged(w: &World, n: &str, before: impl FnOnce(&Obj)) -> Obj {
    w.script(json!({ "worker": [{ "files": { format!("{n}.txt"): "x\n" }, "commit": n, "note": "Did it." }] }));
    let id = w.add(&format!("Task {n}"));
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    before(&t);
    sw::actions::finish_line::approve(&id, None, false).unwrap();
    let c = settle(&id);
    assert_eq!(c.s("stage"), "done", "{}", c.at("failure"));
    c
}
/// The words a flag of the agent's pane gives (`child`, `tool`, `job`); the flag is taken off again afterwards, as the
/// fake's processes are one set for every pane.
fn with(w: &World, flag: &str, value: Value) -> String {
    let t = merged(w, &format!("{flag}-{}", text(&value)), |t| pane(w, t.at("link").s("pane"), |p| p[flag] = value));
    pane(w, t.at("link").s("pane"), |p| p[flag] = json!(false));
    kept(&t)
}

#[test]
fn a_process_beside_the_agent_is_named_with_why_it_keeps_the_space_and_a_helper_wired_to_the_agent_keeps_nothing() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let ends = |s: &str, words: &str| assert!(s.ends_with(&format!("(its space is in use beside its agent: {words})")), "{s}");
    // in the agent's own process group, but not wired to it: what its streams are wired to instead
    ends(&with(&w, "child", json!(true)), "sleep (pid 3) streams to the terminal");
    ends(&with(&w, "child", json!("file")), "sleep (pid 3) is not wired to its agent");
    ends(&with(&w, "child", json!("closed")), "sleep (pid 3) has its input or output closed");
    ends(&with(&w, "child", json!("unknown")), "sleep (pid 3) cannot be looked at");
    // a dev server it started in a group of its own, and a job of the pane's shell: by their command lines
    ends(&with(&w, "tool", json!(true)), "node …/dev-server.js (pid 2) runs in a group of its own");
    ends(&with(&w, "job", json!(true)), "node …/server.js (pid 2) runs in the background of its shell");
    // an MCP server wired to the agent is part of it: nothing is kept
    let t = merged(&w, "helper", |t| pane(&w, t.at("link").s("pane"), |p| p["helper"] = json!(true)));
    assert!(!t.b("kept") && t.b("cleaned"), "{}", kept(&t));
}

#[test]
fn a_turn_an_agent_herdr_cannot_place_and_another_pane_each_give_their_words_and_details_lists_every_thing() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    let status = |t: &Obj, st: &str| w.patch(|s| { let seq = s.i("seq") + 1; s["seq"] = json!(seq); let p = s["panes"].as_array_mut().unwrap().iter_mut().find(|p| p.s("pane_id") == t.at("link").s("pane")).unwrap(); p["agent"]["status"] = json!(st); p["agent"]["seq"] = json!(seq); });
    let t = merged(&w, "turn", |t| status(t, "working"));
    assert!(kept(&t).ends_with("(its space is in use: a turn in its agent's terminal)"), "{}", kept(&t));
    let t = merged(&w, "unsure", |t| status(t, "unknown"));
    assert!(kept(&t).contains("(its space is in use: herdr cannot tell whether its agent works, and its screen"), "{}", kept(&t));
    // a command of yours in a pane you split off in its space
    let t = merged(&w, "split", |t| { let mine = sw::herdr::Herdr::new(None).pane_split(t.at("link").s("pane"), t.s("worktree"), "right", &[]).unwrap().t("pane_id"); pane(&w, &mine, |p| p["busy"] = json!(true)); });
    assert!(kept(&t).ends_with("(its space is in use in another of its panes: sleep (pid 4194401) runs in it)"), "{}", kept(&t));
    let d = sw::actions::finish_line::details(t.s("id")).unwrap();
    assert_eq!(d.at("inUse"), &json!(["in another of its panes: sleep (pid 4194401) runs in it"]));
    assert_eq!(d.at("kept"), t.at("kept"));
    // herdr does not answer for its agent
    w.script(json!({ "worker": [{ "files": { "q.txt": "x\n" }, "commit": "q", "note": "Did it." }] }));
    let q = w.add("Unanswered");
    delegate(&q);
    assert_eq!(settle(&q).s("stage"), "approval");
    sw::store::tasks::update(&q, |c| { c.put("stage", "done"); Ok(sw::store::tasks::Change::Write) }).unwrap();
    w.patch(|s| s["agentGetFails"] = json!(true));
    sw::land::cleanup_job(&q, "merged");
    w.patch(|s| s["agentGetFails"] = json!(false));
    assert!(kept(&task(&q)).ends_with("(its space is in use: herdr does not answer for its agent)"), "{}", kept(&task(&q)));
}

#[test]
fn the_line_names_the_first_thing_and_how_many_more_details_has_them_all() {
    use sw::land::in_use::InUse;
    let u = InUse { ctx: "beside its agent", what: vec!["node …/a.js (pid 5) streams to the terminal".into(), "tsserver (pid 6) is not wired to its agent".into(), "x (pid 7) runs in a group of its own".into()] };
    assert_eq!(u.line("p/7"), "its space is in use beside its agent: node …/a.js (pid 5) streams to the terminal; and 2 more (herdr-projects details p/7)");
    assert_eq!(u.all()[1], "beside its agent: tsserver (pid 6) is not wired to its agent");
    assert_eq!(sw::herdr::busy::short(&["/usr/local/bin/node".into(), "--max-old-space-size=4096".into(), "/Users/me/.npm/_npx/1/node_modules/tsserver/lib/server.js".into()]), "node …/server.js");
    assert_eq!(sw::herdr::busy::short(&["uv".into(), "tool".into(), "uvx".into()]), "uv tool");
}
