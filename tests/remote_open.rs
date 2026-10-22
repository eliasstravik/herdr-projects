//! Enter on another machine's task, from a board in a herdr window: that machine reopens the task's space and its
//! herdr puts the agent in front, as Enter does with an agent of this machine; the board stays, says to pick that
//! machine in herdr's sidebar, and this terminal never becomes the agent's. Outside herdr, and with a machine on an
//! older plugin, the terminal attaches to it as before. Also that machine's side of it: its door focuses the pane.
use crate::common;
use crate::far;
use common::board::{open, Board};
use common::*;
use far::Far;
use serde_json::{json, Value};
use sw::board::job;
use sw::board::keys::Press;
use sw::board::update::{update, Effect, Event};
use sw::json::*;

fn set_herdr(on: bool) { if on { unsafe { std::env::set_var("HERDR_ENV", "1") } } else { unsafe { std::env::remove_var("HERDR_ENV") } } }
fn theirs() -> Value { let now = sw::clock::now(); json!({ "id": "far/1", "title": "Theirs", "stage": "working", "state": "working", "agent": "claude", "app": "claude", "msg": "", "note": "", "at": now, "created": now, "link": { "pane": "w1:p1" }, "snap": { "folder": "~/far" } }) }
/// A board that sees vm-a's tasks, on its one task.
fn board_on_theirs(w: &World) -> Board {
    let mut b = open(&w.root, 34);
    update(&mut b.s, Event::Machines(sw::remote::machines()));
    update(&mut b.s, Event::Remote { label: "vm-a".into(), answer: Ok(json!({ "host": "vm-a", "version": "0.2.0", "tasks": [theirs()] })) });
    b.press(&["alt+m"]);
    b.s.sel = "vm-a:far/1".into(); b.load();
    b
}
/// The open key, with what it started run at once: what this terminal was handed to (an attach's last word) is returned.
fn enter(b: &mut Board) -> Vec<String> {
    let mut went = vec![];
    let mut fx = update(&mut b.s, Event::Key(Press::chord("alt+o")));
    while let Some(e) = fx.pop() {
        match e {
            Effect::Run(j) => { let d = job::run(j); fx.extend(update(&mut b.s, Event::Done(d))); }
            Effect::Attach { args, .. } => went.push(args.last().cloned().unwrap_or_default()),
            _ => {}
        }
    }
    went
}
fn reopen_asked(far: &Far) -> Value { far.asked().into_iter().filter(|a| a.1 == "do").last().map(|a| a.2).unwrap_or(Value::Null) }

#[test]
fn enter_on_another_machines_agent_has_its_herdr_put_it_in_front_and_never_attaches_this_terminal() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-a" }), ..Default::default() });
    let far = Far::new(&w.base);
    far.answers("vm-a", "do.reopen", &json!({ "ok": true, "msg": "", "pane": "w3:p2", "workspace": "w3", "hasAgent": true, "focused": true }));
    // In a herdr window: vm-a is asked to put it in front, the board stays where it is and says where it is.
    set_herdr(true);
    let mut b = board_on_theirs(&w);
    let went = enter(&mut b);
    assert!(went.is_empty(), "this terminal is not the agent's: {went:?}");
    let asked = reopen_asked(&far);
    assert_eq!((asked.s("action"), asked.s("id"), asked.at("focus")), ("reopen", "far/1", &json!(true)));
    assert_eq!(b.s.msg, "vm-a: its agent is in front there. Pick vm-a in herdr's sidebar to go to it: “Theirs”; Local there brings you back here.");
    assert!(!b.closed); assert_eq!(b.s.sel, "vm-a:far/1");
    // Its agent had exited and could not be restarted: its bare terminal is what is in front.
    far.answers("vm-a", "do.reopen", &json!({ "ok": true, "msg": "", "pane": "w3:p2", "workspace": "w3", "hasAgent": false, "focused": true }));
    assert!(enter(&mut b).is_empty());
    assert!(b.s.msg.starts_with("vm-a: its terminal (its agent has exited) is in front there."), "{}", b.s.msg);
    // vm-a's herdr refused: why is said, and nothing else happens.
    far.answers("vm-a", "do.reopen", &json!({ "ok": false, "msg": "vm-a's herdr did not go to it: no such pane" }));
    assert!(enter(&mut b).is_empty());
    assert_eq!(b.s.msg, "vm-a's herdr did not go to it: no such pane");
    // A machine on an older plugin does not know `focus`: this terminal attaches to the agent there, as it did.
    far.answers("vm-a", "do.reopen", &json!({ "ok": true, "msg": "", "pane": "w3:p2", "workspace": "w3", "hasAgent": true }));
    let went = enter(&mut b);
    assert_eq!(went.len(), 1); assert!(went[0].ends_with("herdr agent attach w3:p2"), "{went:?}");
    // Outside a herdr client (plain SSH) there is no window to put it in front of: the terminal attaches.
    set_herdr(false);
    let mut b = board_on_theirs(&w);
    let went = enter(&mut b);
    assert_eq!(reopen_asked(&far).at("focus"), &json!(false));
    assert_eq!(went.len(), 1); assert!(went[0].ends_with("herdr agent attach w3:p2"), "{went:?}");
}

#[test]
fn the_machine_asked_to_reopen_a_task_puts_its_agent_in_front_in_its_own_herdr_only_when_asked_to() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Editing greet.sh\n" }] }));
    let id = w.add("Add a greeting");
    delegate(&id); settle(&id);
    let pane = task(&id).at("link").t("pane");
    let focus = |w: &World| -> Vec<String> { w.state().arr("focus").iter().map(text).collect() };
    let before = focus(&w).len();
    // Asked for the pane only (outside herdr, an older board): nothing moves here.
    let a = sw::remote::door("do", &json!({ "action": "reopen", "id": id })).unwrap();
    assert!(a.b("ok") && a.b("hasAgent")); assert_eq!(a.s("pane"), pane); assert!(!a.b("focused"));
    assert_eq!(focus(&w).len(), before);
    // Asked to put it in front: this machine's herdr goes to the agent, as Enter on this machine's board does.
    let a = sw::remote::door("do", &json!({ "action": "reopen", "id": id, "focus": true })).unwrap();
    assert!(a.b("ok") && a.b("focused"), "{a}");
    assert_eq!(focus(&w)[before..].to_vec(), vec![format!("agent {pane}")]);
}
