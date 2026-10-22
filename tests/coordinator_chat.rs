//! One chat for everyone: space on a task talks to its agent, otherwise to the Coordinator; alt+c only shows or hides
//! the latest message; a draft left from the Coordinator's own pane is sent from the repo it was typed in; and the herdr
//! action that goes into the Coordinator's pane from anywhere, starting it first.
use crate::common;
use common::board::{open, Board};
use common::*;
use serde_json::{json, Value};
use sw::board::keys::key_name;
use sw::coordinator::{look::look, requests::send, state};
use sw::json::*;

fn coord_world() -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn agents(w: &World) -> Vec<Value> { w.state().arr("panes").iter().filter(|p| p.at("agent").is_object()).cloned().collect() }
fn get(w: &World) -> Obj { state::get(&w.socket).unwrap_or_default() }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn wide(b: &mut Board, cols: usize) -> String { b.screen(cols).join("\n") }
const TITLE: &str = "Add a greeting that is polite, friendly and never cut short";

#[test]
fn one_chat_for_everyone_space_on_a_task_talks_to_its_agent_otherwise_to_the_coordinator_alt_c_only_shows_or_hides() {
    let w = coord_world();
    unsafe { std::env::set_var("HERDR_ENV", "1") };
    let so = w.socket.clone();
    let chat_key = regex::escape(&key_name("chatMessage"));
    let top = regex::escape(&crate::board_chat::top_name());
    let mut s = open(&w.root, 34);
    assert!(s.s.chat_open);
    assert!(has(&s.text(), &format!(r"\n╭─ [^\n]*╮\n(?:│ +│\n)?│ +❯ Ask it something, e\.g\. ")) && !s.text().contains(&format!("{} Not started", crate::board_chat::top_name())), "{}", s.text());
    // The board has no key that goes into the Coordinator's pane, restarts it or starts a new one.
    s.press(&["alt+c"]); assert!(!s.s.chat_open); s.press(&["alt+c"]); assert!(s.s.chat_open);
    s.press(&["alt+v", "alt+r", "alt+n"]); assert!(s.s.chat_open); assert!(s.s.ask.is_none()); assert_eq!(s.s.value(), "");
    drain();
    assert!(!s.closed); assert_eq!(look(&so).state, "none"); assert!(!get(&w).b("want"));
    // Back on the board from the Coordinator's pane: herdr names that pane's folder, which is nobody's repo. The board is
    // where it was, so the draft that comes back is sent from the repo it was typed in.
    {
        let mut b = open(&w.root, 34); b.typed("a draft"); sw::board::run::prefs::save(&b.s);
        let home = state::home(&so);
        let st = sw::board::run::start::view(sw::board::state::Ctx { cwd: home.display().to_string(), can_reload: true, in_herdr: true, ..Default::default() }, &home);
        let mut back = Board { s: st, rows: 34, cols: 120, closed: false, theme: sw::board::theme::Theme::default_dark(true) };
        back.load();
        assert_eq!(back.s.value(), "a draft"); assert_eq!(back.s.ctx.cwd, w.root_str()); assert_eq!(back.s.data.repo_root, w.root_str());
        back.press(&["enter"]); drain();
        assert_eq!(get(&w).arr("where").last().unwrap().s("root"), w.root_str(), "not the Coordinator's own folder");
        sw::board::run::prefs::save(&back.s); // closed: the draft it sent is not kept
    }
    // Running: alt+c hides its message and shows it again; it goes nowhere.
    let mut e = open(&w.root, 34);
    e.press(&["alt+c"]); assert!(!e.s.chat_open); e.press(&["alt+c"]); assert!(e.s.chat_open); assert!(!e.closed); assert!(w.state().arr("focus").is_empty());
    // Space on a task: the same chat, now with that task's agent; in the box the agent's last message, which the task
    // panel no longer repeats.
    let t = w.add(TITLE); delegate(&t); settle(&t);
    e.load(); e.s.sel = t.clone(); e.load();
    assert!(has(&wide(&mut e, 160), r"The agent said ─+ *\n[^\n]*Done\."), "{}", wide(&mut e, 160)); assert!(has(&wide(&mut e, 160), r"\n╭─  [^›\n]+  · Machine coordinator ─*╮\n(?:│ +│\n)?"));
    e.press(&["space"]); assert!(e.s.reply);
    let text = wide(&mut e, 160);
    // the box names it: the ladder down to its tag; inside, what it said, under no label
    assert!(has(&text, r"\n╭─ [^\n]*PRJ-1  · Agent ─*╮\n(?:│ +│\n)?│ +Done\. +│\n│ +│\n│ +❯ Send message to the agent +│\n(?:│ +│\n)?╰─+╯\n"), "{text}");
    let lines: Vec<&str> = text.lines().collect();
    assert!(!has(&lines[lines.len() - 8..lines.len() - 1].join("\n"), &format!("its agent|claude|last message|{chat_key}")), "the key is on the hint line, under the box");
    assert!(!has(&text, r"The agent said|\n[ ▌][^ ] Coordinator {2}|reply to")); // (the chat's line, with its mark: not the main Coordinator's row on top) assert!(has(&text, r"space back to the Coordinator *$"), "{text}"); assert_eq!(text.matches("Done.").count(), 1);
    e.press(&["alt+c"]);
    assert!(has(&wide(&mut e, 160), r"The agent said ─+ *\n[^\n]*Done\.")); assert!(has(&wide(&mut e, 160), r"\n╭─ [^\n]*PRJ-\d+  · Agent ─*╮\n(?:│ +│\n)?│ +❯ Send message to the agent"), "{}", wide(&mut e, 160));
    e.press(&["alt+c"]);
    // A narrow window: the box still names its agent, on its edge.
    {
        let lines = e.screen(44);
        assert!(lines.iter().any(|l| has(l, r"^╭─ .*PRJ-1 .*╮$")), "{lines:?}");
        assert!(lines.join("\n").contains("❯ Send message to the agent"));
    }
    // While you talk to a task's agent the chat stays that agent's: the Coordinator's reply waits in its own chat (no
    // green line under the chat says it: the board says nothing in green).
    w.script(json!({ "coordinator": [{ "said": "All done." }] }));
    send(&w.root, "do something", false, "", &so).unwrap(); drain(); e.load();
    assert!(!e.text().contains("The Coordinator replied."), "{}", e.text()); assert!(!e.text().contains("All done.")); assert!(!look(&so).unseen.is_empty());
    // What you type there is for that agent, not for the Coordinator.
    e.typed("also say goodbye"); assert!(e.text().contains("enter send to PRJ-1's agent · shift+enter new line · esc put away")); e.press(&["esc"]);
    // esc is not a chat key: the agent's message stays up, you still talk to the agent, and nothing asks to close.
    e.press(&["esc", "esc"]); assert!(e.s.chat_open && e.s.reply && e.s.ask.is_none() && !e.closed);
    assert!(has(&wide(&mut e, 160), r"\n│ +Done\. +│\n│ +│\n│ +❯ Send message to the agent")); e.s.closing = None;
    // The one key hides and shows a task agent's message exactly as it does the Coordinator's.
    e.press(&["alt+c"]); assert!(!e.s.chat_open && e.s.reply);
    assert!(!has(&wide(&mut e, 160), r"\n│ +Done\. +│\n│ +│\n│ +❯ Send message to the agent")); assert!(has(&wide(&mut e, 160), &format!("{chat_key} show the latest message")));
    // Hidden, the chat still says who you talk to, and that follows the selection: a message is never sent blind.
    {
        let other = w.add("Somebody else"); e.load();
        let order: Vec<String> = e.s.rows.iter().filter_map(|r| match r { sw::board::state::Row::Rec(o) => Some(o.t("id")), sw::board::state::Row::Head { id, .. } => Some(id.clone()), _ => None }).collect(); // the arrows stop on headings too
        let d = order.iter().position(|x| *x == other).unwrap() as i64 - order.iter().position(|x| *x == t).unwrap() as i64;
        for _ in 0..d.abs() { e.press(&[if d > 0 { "down" } else { "up" }]); }
        assert_eq!(e.s.sel, other); assert!(e.s.reply && !e.s.chat_open);
        assert!(has(&wide(&mut e, 160), r"\n╭─ [^\n]*PRJ-\d+  · Agent ─*╮\n(?:│ +│\n)?│ +❯ your message starts an agent on it"), "{}", wide(&mut e, 160));
        for _ in 0..d.abs() { e.press(&[if d > 0 { "up" } else { "down" }]); }
        assert_eq!(e.s.sel, t);
        assert!(has(&wide(&mut e, 160), &format!(r"\n╭─ [^\n]*PRJ-\d+  · Agent ─*╮\n(?:│ +│\n)?│ +❯ Send message to the agent +│\n(?:│ +│\n)?╰─+╯\n[^\n]*{chat_key} show the latest message")), "{}", wide(&mut e, 160));
        sw::actions::archive::clear(&other).unwrap(); e.load();
    }
    e.press(&["alt+c"]); assert!(e.s.chat_open); assert!(has(&wide(&mut e, 160), r"\n│ +Done\. +│\n│ +│\n│ +❯ Send message to the agent"));
    // space goes back to the top row's coordinator with its message up, and its reply is there.
    e.press(&["space"]); assert!(e.s.chat_open && !e.s.reply); e.load();
    assert!(has(&e.text(), &format!(r"╭─[^\n]*╮\n(?:│ +│\n)?│ +All done\. +│\n│ +│\n│ +❯ Send message to the machine coordinator")), "{}", e.text()); assert_eq!(look(&so).unseen, "");
    // Nothing changes who the chat names while you type.
    e.typed("half typed");
    w.script(json!({ "coordinator": [{ "said": "Again." }] }));
    send(&w.root, "once more", false, "", &so).unwrap(); drain(); e.load(); e.load();
    assert!(has(&e.text(), &format!(r"╭─[^\n]*╮\n(?:│ +│\n)?│ +Again\. +│\n│ +│\n│ +❯ half typed")), "{}", e.text());
    e.press(&["esc"]);
    unsafe { std::env::remove_var("HERDR_ENV") };
}

// (The old command waited for the start in the same process while the test ran the jobs beside it. Here the jobs run in
// the test's own thread: the first go asks for the start and does not wait; once it ran, the action goes in.)
#[test]
fn the_herdr_action_goes_into_the_coordinators_pane_from_anywhere_starting_it_first_when_it_is_not_running() {
    let w = coord_world();
    let so = w.socket.clone();
    let first = sw::coordinator::look::go_in(&so, -1);
    assert!(first.is_err(), "it asked for the start and did not wait here");
    assert!(sw::runner::JOBS.with(|j| j.borrow().iter().any(|x| x.0 == "coordinator")));
    drain();
    assert_eq!(sw::cli::main(vec!["open-coordinator".into()]), 0);
    assert_eq!(agents(&w).len(), 1); assert_eq!(w.state().arr("focus").iter().map(text).collect::<Vec<_>>(), vec![format!("agent {}", get(&w).s("pane"))], "its pane, not the board");
    assert_eq!(sw::cli::main(vec!["open-coordinator".into()]), 0); assert_eq!(w.state().arr("focus").len(), 2);
    assert!(agents(&w)[0].at("agent").arr("prompts").is_empty(), "nothing was said to it");
}
