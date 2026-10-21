//! More of the Coordinator's chat: esc never shows, hides or closes it (only alt+c does), a task with no agent is talked
//! about to the Coordinator, enter goes into a task's agent, the chat's head line at every width, a narrow or short
//! window; and a message sent with tab, whose tasks the board itself delegates when they come from that turn.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::board::keys::key_name;
use sw::coordinator::{requests::{send, tab_note, where_text}, rules::rules, state};
use sw::json::*;

fn coord_world() -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude", "review.when": "never" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    unsafe { std::env::remove_var("HERDR_PROJECTS_HANDLED_MS") };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn prompts_of(w: &World) -> Vec<String> { w.state().arr("panes").iter().find(|p| p.at("agent").is_object()).map(|p| p.at("agent").arr("prompts").iter().map(text).collect()).unwrap_or_default() }
fn get(w: &World) -> Obj { state::get(&w.socket).unwrap_or_default() }
fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn cli(args: &[&str]) -> String { let a = sw::cli::flags::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()); sw::cli::tasks::command(&a.pos[0], &a).unwrap_or_else(|e| panic!("{args:?}: {}", e.msg)).1 }

#[test]
fn esc_hides_but_never_shows_the_chat_and_a_task_with_no_agent_is_talked_about_to_the_coordinator() {
    let w = coord_world();
    let chat_key = regex::escape(&key_name("chatMessage"));
    w.script(json!({ "coordinator": [{ "said": "Added proj/1." }, { "said": "Delegated proj/1." }, { "said": "Shorter now." }] }));
    let mut s = open(&w.root, 34);
    s.press(&["?"]); assert!(has(&s.text(), &format!("{chat_key} +hide the chat's latest message")), "{}", s.text()); assert!(!has(&s.text(), "restart the Coordinator|new Coordinator|Coordinator's chat")); s.press(&["esc"]);
    s.typed("build a login page"); s.press(&["enter"]); drain(); s.load();
    assert!(s.text().contains("Added proj/1."));
    // Hidden: who you talk to, and the line you type on.
    s.press(&["alt+c"]);
    assert!(has(&s.text(), &format!(r"\n *\n[ ▌]○ Coordinator {{2}}idle *\n─[^\n]*\n❯ Send message to Coordinator *\n─+\n[ ▌] [^\n]*{chat_key} show the latest message · \? all keys *$")), "{}", s.text());
    // esc never shows the message, and never closes the board: hidden, it stays hidden, and nothing is asked.
    s.press(&["esc"]); assert!(!s.s.chat_open && s.s.ask.is_none() && !s.closed && s.s.closing.is_some());
    assert!(has(&s.text(), &format!(r"\n  {} closes the board · the Coordinator keeps running *$", regex::escape(&key_name("close")))), "{}", s.text()); s.s.closing = None;
    // Shown, with nothing else to step back from, esc hides it as its own key does (the chat's line names only that key).
    s.press(&["alt+c"]); assert!(s.s.chat_open && s.text().contains("Added proj/1.")); assert!(has(&s.text(), &format!("{chat_key} hide the latest message"))); assert!(!s.text().contains("esc hide"));
    s.press(&["esc"]); assert!(!s.s.chat_open && s.s.ask.is_none() && !s.closed && s.s.closing.is_none() && !s.text().contains("Added proj/1."));
    assert_eq!(sw::store::files::read_obj(&sw::store::files::data_dir().join("view.json")).unwrap().s("chat"), "closed");
    s.press(&["alt+c"]); assert!(s.s.chat_open && s.text().contains("Added proj/1."));
    // With a draft, esc puts the words away and the message stays up; only the next esc hides it.
    s.typed("half");
    assert!(has(&s.text(), r"│ Added proj/1\. +│\n│ +│\n│ ❯ half +│\n╰─+╯\n[ ▌] enter send to the Coordinator · shift\+enter new line · esc put away *$"), "{}", s.text());
    s.press(&["esc"]); assert_eq!(s.s.value(), ""); assert!(s.s.chat_open);
    s.text(); s.press(&["esc"]); assert!(!s.s.chat_open && s.s.closing.is_none());
    s.typed("and start it"); s.press(&["enter"]); drain();
    assert_eq!(get(&w).s("last"), "and start it"); assert_eq!(get(&w).s("over"), "Added proj/1.", "what it had last said when the words went in");
    // With a task open that has no agent to reply to, the words go to the Coordinator with that task named.
    let t = w.add("Write the docs");
    sw::actions::filing::report(&t, sw::actions::filing::Report { state: "done".into(), ..Default::default() }).unwrap();
    s.load(); s.s.sel = t.clone(); s.load(); s.press(&["space"]); assert!(s.s.reply);
    let txt = s.screen(160).join("\n");
    assert!(has(&txt, r"\n *\n[ ▌]. PRJ-\d+ Write the docs {2}no agent on it: your message goes to the Coordinator, about this task *\n─[^\n]*\n❯ Send message to Coordinator"), "{txt}");
    s.typed("make it shorter"); s.press(&["enter"]); drain();
    assert_eq!(prompts_of(&w)[2], "make it shorter");
    assert!(has(&where_text(&w.socket), r#"\n- "make it shorter"[^\n]*Task proj/1 \("Write the docs"\) was open: for this message "it" and "this" mean that task\. It was sent with enter"#), "{}", where_text(&w.socket));
    assert!(cli(&["coordinator", "where"]).contains("was sent from the repo \"proj\""), "the Coordinator's own command says it");
}

#[test]
fn sent_with_tab_every_task_the_coordinator_adds_from_that_message_gets_an_agent_said_in_the_message_and_done_by_the_board_itself() {
    let w = coord_world();
    let root = w.root_str();
    let board = |args: &[&str]| { let mut a: Vec<&str> = args.to_vec(); a.extend(["--repo", &root]); let out = cli(&a); sw::runner::JOBS.with(|j| j.borrow_mut().clear()); out }; // (the agents themselves are not started here)
    let started = |id: &str| ["queued", "starting"].contains(&task(id).s("stage"));
    let ids = || { let mut v: Vec<String> = sw::store::tasks::list(Default::default()).iter().map(|t| t.t("id")).collect(); v.sort_by_key(|x| x.split('/').nth(1).and_then(|n| n.parse::<i64>().ok()).unwrap_or(0)); v };
    let on = || ids().into_iter().filter(|i| started(i)).collect::<Vec<_>>();
    // The Coordinator ends its turn: herdr counts one more finished turn.
    let end_turn = || w.patch(|st| { let seq = st.i("seq") + 1; st["seq"] = json!(seq); for p in st["panes"].as_array_mut().unwrap() { if p.at("agent").is_object() { p["agent"]["status"] = json!("done"); p["agent"]["seq"] = json!(seq); p["agent"]["completion"] = json!(seq); } } });
    let pane_env = |p: Option<&str>| unsafe { match p { Some(p) => std::env::set_var("HERDR_PANE_ID", p), None => std::env::remove_var("HERDR_PANE_ID") } };
    unsafe { std::env::set_var("HERDR_PROJECTS_HANDLED_MS", "0") };
    // tab: the message itself says so, under the words. The Coordinator is now in that message's turn.
    w.script(json!({ "coordinator": [{ "status": "working" }] }));
    send(&w.root, "a login page, and dark mode", true, "", &w.socket).unwrap(); drain();
    assert_eq!(prompts_of(&w)[0], format!("a login page, and dark mode\n\n{}", tab_note()));
    assert!(has(&rules("board"), r"A message sent with tab ends with a line in square brackets saying so\.[^\n]*every task you add from that message gets an agent at once, with no question back"));
    let pane = get(&w).t("pane");
    // A task added from anywhere else is only added: the human's shell, a working agent's pane.
    assert_eq!(board(&["add", "Mine"]), "proj/1: added");
    pane_env(Some("w9:p1"));
    assert_eq!(board(&["add", "A worker's follow-up"]), "proj/2: added");
    // Added from the Coordinator's pane, with no flag at all (it forgot): the board delegates it. Every one of them.
    pane_env(Some(&pane));
    assert_eq!(board(&["add", "Login page"]), "proj/3: added, an agent is on it (the message was sent with tab)");
    assert_eq!(board(&["add", "Dark mode"]), "proj/4: added, an agent is on it (the message was sent with tab)");
    // It remembered the flag: the same, once.
    assert_eq!(board(&["add", "Sign up", "--delegate"]), "proj/5: added, an agent is on it");
    assert_eq!(on(), vec!["proj/3", "proj/4", "proj/5"]);
    // Its turn is over. What it adds later is not that message's: added, not delegated.
    end_turn();
    assert_eq!(board(&["add", "Later"]), "proj/6: added");
    // A tab message that adds nothing (a question) delegates nothing.
    w.script(json!({ "coordinator": [{ "said": "Two are being worked on." }] }));
    send(&w.root, "how is it going?", true, "", &w.socket).unwrap(); drain();
    assert_eq!(ids().len(), 6); assert_eq!(on(), vec!["proj/3", "proj/4", "proj/5"]);
    // enter: as before. The words go in alone, and what is added from them is only added.
    w.script(json!({ "coordinator": [{ "status": "working" }] }));
    send(&w.root, "note down: tidy the footer", false, "", &w.socket).unwrap(); drain();
    assert_eq!(prompts_of(&w).last().unwrap(), "note down: tidy the footer");
    assert_eq!(board(&["add", "Tidy the footer"]), "proj/7: added");
    // A tab message that goes in while it is still busy with the enter one: the board leaves it to the line in the message.
    w.script(json!({ "coordinator": [{ "status": "working" }] }));
    send(&w.root, "and a new logo", true, "", &w.socket).unwrap(); drain();
    assert_eq!(prompts_of(&w).last().unwrap(), &format!("and a new logo\n\n{}", tab_note()));
    assert_eq!(board(&["add", "New logo"]), "proj/8: added");
    // Both dealt with; the next tab message is clear again.
    end_turn();
    w.script(json!({ "coordinator": [{ "status": "working" }] }));
    send(&w.root, "a pricing page", true, "", &w.socket).unwrap(); drain();
    assert_eq!(board(&["add", "Pricing page"]), "proj/9: added, an agent is on it (the message was sent with tab)");
    // A Coordinator that was restarted is not in that turn any more.
    state::change(&w.socket, |c| { let n = c.i("startedAt") + 1; c.put("startedAt", n); true }).unwrap();
    assert_eq!(board(&["add", "After a restart"]), "proj/10: added");
    assert_eq!(on(), vec!["proj/3", "proj/4", "proj/5", "proj/9"]);
    pane_env(None); unsafe { std::env::remove_var("HERDR_PROJECTS_HANDLED_MS") };
}

#[test]
fn enter_goes_into_a_tasks_agent_the_chats_head_line_at_every_width_and_a_narrow_or_short_window_keeps_the_box_whole() {
    let w = coord_world();
    let chat_key = regex::escape(&key_name("chatMessage"));
    unsafe { std::env::set_var("HERDR_ENV", "1") }; // the board inside herdr
    let title = "Add a greeting that is polite, friendly and never cut short";
    let t = w.add(title); delegate(&t); settle(&t);
    w.add("Somebody else"); sw::actions::archive::clear("proj/2").unwrap(); // the old test's numbers: this one is PRJ-3
    // Into a task's agent is enter, with nothing typed: herdr shows that agent's pane.
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Editing greet.sh\n" }] }));
    let u = w.add("Still at it"); delegate(&u); settle(&u);
    let mut g = open(&w.root, 34); g.s.sel = u.clone(); g.load(); g.press(&["space"]);
    assert!(has(&g.screen(160).join("\n"), r"\n *\n[ ▌]. PRJ-\d+ Still at it {2}working *\n╭─[^\n]*╮\n│ The agent has just started: nothing to show yet\. +│\n│ +│\n│ ❯ Send message to Agent +│\n╰─+╯\n"), "{}", g.screen(160).join("\n"));
    // That line is what herdr's sidebar says about the agent: the row's mark, the name, the Watcher's "doing now" line.
    sw::store::tasks::update(&u, |c| { c.put("msg", "Verifying the test run after the rebase onto main"); Ok(sw::store::tasks::Change::Write) }).unwrap(); g.load();
    let lines = g.screen(160);
    assert!(has(&lines.join("\n"), r"\n[ ▌]. PRJ-\d+ Still at it {2}Verifying the test run after the rebase onto main *\n╭─[^\n]*╮\n│ The agent has just started"), "{}", lines.join("\n"));
    assert!(!has(&lines[lines.len() - 7], &format!("{chat_key}|its agent · ")));
    // However little room the name leaves, the status is cut, never dropped: down to the "…" alone. The task's number
    // stands before its name while the status keeps a readable width beside it; then it goes.
    let head = |g: &mut common::board::Board, cols: usize| { let l = g.screen(cols); let at = l.iter().position(|x| x.starts_with("╭─")).unwrap(); l[at - 1].clone() };
    let mark: String = head(&mut g, 160).chars().take(2).collect();
    for (cols, want) in [(40, " PRJ-3 Still at it  Verifying the tes…"), (29, " PRJ-3 Still at it  Verify…"), (28, " Still at it  Verifying t…"), (25, " Still at it  Verifyin…"), (23, " Still at it  Verify…"), (18, " Still at it  V…"), (17, " Still at it  …"), (16, " Still at it")] {
        assert_eq!(head(&mut g, cols), format!("{mark}{want}"), "at {cols}");
    }
    g.press(&["enter"]); assert!(!g.closed); assert_eq!(text(w.state().arr("focus").last().unwrap()), format!("agent {}", task(&u).at("link").s("pane"))); // into its agent: the board stays
    // A narrow window is the same chat: space talks to the task's agent and opens nothing. On the whole task (alt+t) the
    // chat folds to its head line. Space again goes back to the Coordinator.
    {
        let mut n = open(&w.root, 40); n.s.sel = t.clone(); n.load(); n.screen(70); n.press(&["space"]);
        assert!(n.s.reply && !n.s.full); n.press(&["alt+t"]);
        assert!(has(&n.screen(70).join("\n"), r"\n─+\n❯ Send message to Agent *\n─+\n[ ▌] enter open this task's agent · esc back to the list *$"), "{}", n.screen(70).join("\n"));
        assert!(!has(&n.screen(90).join("\n"), &chat_key), "the chat's key is not offered where the chat is folded");
        assert!(has(&n.screen(70).join("\n"), r"The agent said ─+ *\n Done\."));
        n.press(&["alt+c"]); assert!(has(&n.screen(70).join("\n"), r"The agent said ─+ *\n Done\."));
        // esc leaves the whole task and nothing else: the chat stays hidden, and stays the agent's.
        n.press(&["esc"]); assert!(!n.s.full && n.s.reply && !n.s.chat_open);
        n.press(&["alt+c", "space"]); assert!(!n.s.reply && !n.s.full); assert!(has(&n.screen(70).join("\n"), r"\n[ ▌]. Coordinator {2}"));
        // alt+t alone never changes who gets your words.
        n.press(&["alt+t"]); assert!(n.s.full && !n.s.reply); assert!(has(&n.screen(70).join("\n"), r"\n[ ▌]. Coordinator {2}[^\n]*\n─+\n❯ Send message to Coordinator"), "{}", n.screen(70).join("\n"));
        n.typed("for the Coordinator"); n.press(&["enter"]);
        assert_eq!(get(&w).arr("queue").last().unwrap().s("text"), "for the Coordinator"); drain();
    }
    // A short, narrow window with a long name and a draft of several lines: the draft's line, the box and the hints are whole.
    {
        let mut x = open(&w.root, 8); x.s.sel = t.clone(); x.load(); x.screen(44); x.press(&["space"]);
        x.s.input.set_text("one\ntwo\nthree\nfour and last");
        let lines = x.screen(44);
        assert_eq!(lines.len(), 8); assert!(lines.iter().all(|l| sw::text::width(l) <= 44));
        assert!(lines.iter().any(|l| l.contains("four and last")), "the line you are typing shows: {lines:?}"); assert!(lines.last().unwrap().contains("enter send to PRJ-1's agent")); assert!(has(&lines[lines.len() - 2], "^─+$"));
        assert!(lines.iter().any(|l| l.contains("Add a greeting") && !l.contains("task panel")), "and so does who you talk to");
    }
    for width in [24, 40, 64, 90, 200] { for reply in [false, true] { let mut x = open(&w.root, 34); x.s.sel = t.clone(); x.load(); if reply { x.press(&["space"]); } let l = x.screen(width); assert!(l.iter().all(|l| sw::text::width(l) <= width), "fits {width}"); assert_eq!(l.len(), 34); } }
    for rows in [8, 12, 20] {
        let mut x = open(&w.root, rows); let l = x.screen(80);
        assert_eq!(l.len(), rows); assert!(l.iter().any(|l| has(l, "^(│ )?❯")), "the text box is never pushed off a short window");
        assert_eq!(l.iter().any(|l| l.starts_with("│ ❯")), rows >= 14, "under 14 rows the chat folds to its head line, and the list has the lines");
    }
    unsafe { std::env::remove_var("HERDR_ENV") };
}
