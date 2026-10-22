//! The popup never left behind: its board closes the moment herdr shows another tab. In the popup the board key itself
//! closes the board. The popup opens on the task of the pane you were in, and words left in its box stay with the repo
//! they were typed for. With the tab chosen, the key pressed on the board goes back to where you came from.
use crate::common;
use common::board::Board;
use common::*;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use sw::board::place::{arrive, show};
use sw::board::state::{Ctx, State};
use sw::board::update::list::relist;
use sw::json::*;

fn set_env(pairs: &[(&str, &str)]) { for k in ["HERDR_ENV", "HERDR_PANE_ID", "HERDR_PLUGIN_ID", "HERDR_PLUGIN_CONTEXT_JSON"] { unsafe { std::env::remove_var(k) } } for (k, v) in pairs { unsafe { std::env::set_var(k, v) } } }
fn here(pane: &str, tab: &str) -> String { json!({ "focused_pane_id": pane, "tab_id": tab }).to_string() }
fn flat(t: &str) -> String { t.split_whitespace().collect::<Vec<_>>().join(" ") }
fn view_write(name: &str, v: Value) { sw::store::files::write_json(&sw::store::files::data_dir().join(name), &v).unwrap(); }
fn view(name: &str) -> Obj { sw::store::files::read_obj(&sw::store::files::data_dir().join(name)).unwrap_or_default() }
/// A board opening as the program opens it (start.rs: view), with what it was told: `popup`, the pane it came `from`, its folder.
fn opened(cwd: &std::path::Path, popup: bool, from: &str, own_key: Option<(Vec<String>, String)>) -> Board {
    let ctx = Ctx { cwd: cwd.display().to_string(), popup, from: from.into(), own_key, can_reload: true, ..Default::default() };
    let s: State = sw::board::run::start::view(ctx, cwd);
    let mut b = Board { s, rows: 34, cols: 120, closed: false, theme: sw::board::theme::Theme::default_dark(true) };
    b.load(); relist(&mut b.s); b.load();
    b
}
fn close(b: &Board) { sw::board::run::prefs::save(&b.s); }

/// herdr's socket, for the subscription and the one question the board asks once herdr has taken it: each connection
/// is answered in turn, then the tabs that come to show are told on the first.
fn herdr_socket(path: &std::path::Path, shows: Value, tabs: &'static [&'static str]) -> std::thread::JoinHandle<Vec<Value>> {
    let l = std::os::unix::net::UnixListener::bind(path).unwrap();
    std::thread::spawn(move || {
        let mut asked = vec![];
        let (mut sub, _) = l.accept().unwrap();
        let mut line = String::new(); BufReader::new(sub.try_clone().unwrap()).read_line(&mut line).unwrap();
        asked.push(serde_json::from_str::<Value>(&line).unwrap());
        sub.write_all(format!("{}\n", json!({ "id": "x", "result": { "type": "subscription_started" } })).as_bytes()).unwrap();
        let (mut q, _) = l.accept().unwrap();
        let mut line = String::new(); BufReader::new(q.try_clone().unwrap()).read_line(&mut line).unwrap();
        asked.push(serde_json::from_str::<Value>(&line).unwrap());
        let a = if shows.is_null() { json!({ "id": "x", "error": { "code": "tab_not_found", "message": "no" } }) } else { json!({ "id": "x", "result": shows }) };
        q.write_all(format!("{a}\n").as_bytes()).unwrap(); drop(q);
        for t in tabs { sub.write_all(format!("{}\n", json!({ "event": "tab_focused", "data": { "tab_id": t } })).as_bytes()).unwrap(); }
        asked
    })
}
/// How many times the board watching `tab` was told to close, once herdr's socket has said all it says.
fn closes(w: &World, n: usize, tab: &str, shows: Value, tabs: &'static [&'static str]) -> (usize, Vec<Value>) {
    let path = w.base.join(format!("events-{n}.sock"));
    let server = herdr_socket(&path, shows, tabs);
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let tx = std::sync::Mutex::new(tx);
    sw::board::run::start::watch_tab(Some(path.display().to_string()), tab, move || { let _ = tx.lock().unwrap().send(()); });
    let asked = server.join().unwrap();
    // The connection ends with the server: the watch ends, and with it what it was told to call.
    (rx.iter().count(), asked)
}

#[test]
fn the_popup_is_never_left_behind_its_board_closes_the_moment_herdr_shows_another_tab() {
    let w = world();
    // herdr tells of every tab that comes to show; its own tab showing again is nothing, another tab of the space and
    // another space's tab are.
    let (n, asked) = closes(&w, 1, "w9:t1", json!({ "tab": { "tab_id": "w9:t1", "focused": true } }), &["w9:t1", "w9:t2", "w3:t1"]);
    assert_eq!(n, 2);
    assert_eq!(asked[0].at("method"), &json!("events.subscribe")); assert_eq!(asked[0].at("params"), &json!({ "subscriptions": [{ "type": "tab.focused" }] }));
    // The tab changed while the board was still starting, before herdr was asked to tell: once it tells, the board
    // asks whether its tab still shows, and closes when it does not.
    let (n, asked) = closes(&w, 2, "w9:t1", json!({ "tab": { "tab_id": "w9:t1", "focused": false } }), &[]);
    assert_eq!(asked[1].at("method"), &json!("tab.get")); assert_eq!(asked[1].at("params"), &json!({ "tab_id": "w9:t1" })); assert_eq!(n, 1);
    assert_eq!(closes(&w, 3, "w9:t1", json!({ "tab": { "tab_id": "w9:t1", "focused": true } }), &[]).0, 0, "its tab shows: it stays");
    assert_eq!(closes(&w, 4, "w9:t1", Value::Null, &["w9:t1"]).0, 0, "herdr did not say: it stays, and the next tab that shows closes it");
    // A board that was not told its tab watches nothing: nothing is asked of herdr.
    let path = w.base.join("events-5.sock"); let l = std::os::unix::net::UnixListener::bind(&path).unwrap(); l.set_nonblocking(true).unwrap();
    sw::board::run::start::watch_tab(Some(path.display().to_string()), "", || panic!("never"));
    assert!(l.accept().is_err());
}

#[test]
fn in_the_popup_the_board_key_closes_the_board_herdr_hands_the_popup_every_key_its_own_too() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.open": "popup" }), ..Default::default() });
    let first = w.add("First");
    // "prefix+a" is herdr's prefix key and then a: the prefix alone does nothing, as in herdr, and what follows it is
    // only the board key when it is that key.
    let defs = sw::herdr::keys::built_in();
    let presses = |k: &str, conf: &str| sw::herdr::keys::presses(k, conf, &defs);
    let own = presses("prefix+a", "");
    assert_eq!(own, Some((vec!["ctrl+b".to_string()], "a".to_string())));
    let mut s = opened(&w.root, true, "", own.clone());
    s.press(&["ctrl+b"]); assert!(!s.closed); assert_eq!(s.s.value(), "");
    s.typed("x"); assert_eq!(s.s.value(), "x", "another key after the prefix is typed as always"); assert!(!s.closed);
    s.typed("a"); assert_eq!(s.s.value(), "xa", "a alone is a letter");
    s.press(&["ctrl+b"]); s.typed("a"); assert!(s.closed, "closed, with a draft in the box too"); assert_eq!(s.s.value(), "xa");
    assert_eq!(view("view-popup.json").s("draft"), "xa", "and the draft is there next time");
    // The prefix is waited for a moment only: a late a is a letter.
    let mut late = opened(&w.root, true, "", own.clone());
    let t = sw::clock::now(); sw::clock::set(Some(t)); late.press(&["ctrl+b"]); sw::clock::set(Some(t + 60000)); late.typed("a"); sw::clock::set(None); assert!(!late.closed); assert!(late.s.value().ends_with('a'));
    // While a question is asked, the key still closes: it is the way out everywhere.
    view_write("view-popup.json", json!({}));
    let mut q = opened(&w.root, true, "", own.clone()); q.s.sel = first.clone(); q.load(); q.press(&["ctrl+x"]); assert!(q.s.ask.is_some(), "{}", q.text()); q.press(&["ctrl+b"]); q.typed("a"); assert!(q.closed);
    // A key without the prefix is one press; the prefix keys are the config's own, one or several.
    assert_eq!(presses("Ctrl+Alt+B", ""), Some((vec![], "ctrl+alt+b".to_string())));
    assert_eq!(presses("prefix+-", "[keys]\nprefix = [\"ctrl+a\", \"ctrl+space\"]\n"), Some((vec!["ctrl+a".to_string(), "ctrl+space".to_string()], "-".to_string())));
    assert_eq!(presses("", ""), None);
    let mut one = opened(&w.root, true, "", presses("ctrl+g", "")); one.press(&["ctrl+g"]); assert!(one.closed);
    // A board in a pane never sees that key (herdr takes it), and does nothing with its parts.
    view_write("view.json", json!({}));
    let mut pane = opened(&w.root, false, "", own); pane.press(&["ctrl+b"]); pane.typed("a"); assert!(!pane.closed); assert_eq!(pane.s.value(), "a");
}

#[test]
fn the_popup_opens_on_the_task_of_the_pane_you_were_in_and_words_left_in_its_box_stay_with_the_repo_they_were_typed_for() {
    let w = world();
    let root = w.root_str();
    w.patch(|s| { s["workspaces"].as_array_mut().unwrap().push(json!({ "workspace_id": "w9", "label": "b", "cwd": root })); for (pane, tab) in [("w9:p1", "w9:t1"), ("w9:p2", "w9:t2")] { s["panes"].as_array_mut().unwrap().push(json!({ "pane_id": pane, "workspace_id": "w9", "tab_id": tab, "terminal_id": format!("term_{pane}"), "cwd": root, "agent": null })); } });
    set_env(&[("HERDR_ENV", "1")]);
    let (a, b, c) = (w.add("First"), w.add("Second"), w.add("Third"));
    let link = |id: &str, l: Value| sw::store::tasks::update(id, |t| { t.put("link", l.clone()); Ok(sw::store::tasks::Change::Write) }).unwrap();
    link(&b, json!({ "socket": w.socket, "workspace": "w9", "tab": "w9:t1", "pane": "w9:p1" }));
    link(&c, json!({ "socket": w.socket, "workspace": "w9", "tab": "w9:t2", "pane": "w9:p9", "reviewer": { "pane": "w9:p2" } }));
    use sw::board::place::task_at;
    assert_eq!(task_at("w9:p1").unwrap().t("id"), b); assert_eq!(task_at("w9:p2").unwrap().t("id"), c, "its reviewer's pane too"); assert!(task_at("w9:p5").is_none()); assert!(task_at("").is_none());
    view_write("view-popup.json", json!({ "sel": a, "filter": "first" }));
    // From the agent's pane: that task, shown also while the filter left from last time would hide it.
    let mut s = opened(&w.root, true, "w9:p1", None);
    assert_eq!(s.s.sel, b); assert!(s.text().contains("Second"), "{}", s.text());
    // The popup was last looking at one other machine only: this machine's tasks show, or the task would be hidden
    // and the chat would talk to that machine. Every machine at once has the task, and stays.
    view_write("view-popup.json", json!({ "sel": a, "scope": "otherbox" }));
    let mut far = opened(&w.root, true, "w9:p1", None);
    assert_eq!(far.s.scope, ""); assert_eq!(far.s.sel, b); assert!(far.text().contains("Second")); assert_eq!(far.s.current().map(|r| r.t("id")), Some(b.clone()));
    assert_eq!(opened(&w.root, true, "w9:p5", None).s.scope, "otherbox", "from no task's pane the view is as it was left");
    view_write("view-popup.json", json!({ "sel": a, "scope": "*" }));
    let all = opened(&w.root, true, "w9:p1", None); assert_eq!(all.s.scope, "*"); assert_eq!(all.s.sel, b);
    view_write("view-popup.json", json!({ "sel": a, "filter": "first" }));
    // From a pane that is no task's: on the top row, whatever was selected when it closed.
    let o = opened(&w.root, true, "w9:p5", None); assert_eq!(o.s.sel, sw::board::update::top::top_id(&o.s));
    // A board in a pane opens on the top row too, wherever you come from.
    view_write("view.json", json!({ "sel": a }));
    let o = opened(&w.root, false, "w9:p1", None); assert_eq!(o.s.sel, sw::board::update::top::top_id(&o.s));
    // Words left in the popup's box are sent from the repo they were typed in, also when it reopens somewhere else.
    view_write("view-popup.json", json!({}));
    let other = w.base.join("elsewhere-x1"); std::fs::create_dir_all(&other).unwrap();
    let mut typed = opened(&w.root, true, "", None); typed.typed("Fix the greeting"); close(&typed);
    assert_eq!(view("view-popup.json").s("cwd"), root);
    // Reopened from a pane in another folder: the draft is back, and the board is still in the repo it was typed for.
    let mut dr = opened(&other, true, "", None);
    assert_eq!(dr.s.value(), "Fix the greeting"); assert_eq!(dr.s.ctx.cwd, root); assert_eq!(dr.s.data.repo, "proj");
    assert!(flat(&dr.text()).contains("What you typed was for proj, so the board is there now. Once it is sent or put away, it opens where you are."), "{}", dr.text());
    dr.press(&["enter"]);
    let made = sw::store::tasks::list(Default::default()).into_iter().find(|t| t.s("title") == "Fix the greeting").unwrap(); assert_eq!(made.s("root"), root); assert_eq!(made.s("repo"), "proj");
    close(&dr);
    // With nothing left in the box the popup opens where you are.
    assert_eq!(opened(&other, true, "", None).s.ctx.cwd, other.display().to_string());
    // Words put away with esc keep their repo too: brought back in a popup that opened elsewhere, the board goes to
    // the repo they were typed for and says so, and enter sends them from there.
    let mut put = opened(&w.root, true, "", None); put.typed("Tidy the greeting"); put.press(&["esc"]); assert_eq!(put.s.value(), ""); close(&put);
    assert_eq!(view("view-popup.json").at("away").s("cwd"), root);
    let far2 = opened(&other, true, "", None);
    assert_eq!(far2.s.ctx.cwd, other.display().to_string()); assert_eq!(far2.s.value(), "");
    close(&far2); // closed and opened once more elsewhere: the words still know where they are from
    assert_eq!(view("view-popup.json").at("away").s("cwd"), root);
    let mut back = opened(&other, true, "", None);
    back.press(&["ctrl+z"]);
    assert_eq!(back.s.value(), "Tidy the greeting"); assert_eq!(back.s.ctx.cwd, root);
    back.load(); assert_eq!(back.s.data.repo, "proj");
    assert!(flat(&back.text()).contains("What you typed was for proj, so the board is there now."), "{}", back.text()); assert!(back.text().contains("herdr-projects  proj · "), "{}", back.text());
    back.press(&["enter"]);
    let all = sw::store::tasks::list(Default::default());
    assert_eq!(all.iter().find(|t| t.s("title") == "Tidy the greeting").unwrap().s("root"), root); assert!(!all.iter().any(|t| t.s("root") == other.display().to_string()));
    close(&back);
    // The folder the words were typed for is gone (removed, moved): there is no repo to go back to, so the board says
    // where they go now, for a draft in the box and for words brought back alike. Never without a word.
    let lost = w.base.join("was-here").display().to_string();
    let gone = regex::Regex::new(r"What you typed was for was-here, a folder that is gone\. Sent now, it goes from elsewhere-\w+; esc puts it away\.").unwrap();
    view_write("view-popup.json", json!({ "draft": "old words", "cwd": lost }));
    let mut g1 = opened(&other, true, "", None);
    assert_eq!(g1.s.ctx.cwd, other.display().to_string()); assert_eq!(g1.s.value(), "old words"); assert!(gone.is_match(&flat(&g1.text())), "{}", g1.text());
    view_write("view-popup.json", json!({ "away": { "text": "older words", "to": "", "cwd": lost }, "cwd": lost }));
    let mut g2 = opened(&other, true, "", None);
    assert!(!g2.text().contains("folder that is gone")); g2.press(&["ctrl+z"]);
    assert_eq!(g2.s.value(), "older words"); assert_eq!(g2.s.ctx.cwd, other.display().to_string()); assert!(gone.is_match(&flat(&g2.text())), "{}", g2.text());
    // The same repo from another of its folders is no move, and nothing is said. A board in a pane is where you started it.
    let sub = w.root.join("sub"); std::fs::create_dir_all(&sub).unwrap();
    view_write("view-popup.json", json!({ "draft": "more", "cwd": root }));
    assert!(!opened(&sub, true, "", None).text().contains("What you typed was for"));
    view_write("view.json", json!({ "draft": "more", "cwd": root }));
    assert_eq!(opened(&other, false, "", None).s.ctx.cwd, other.display().to_string());
    set_env(&[]);
}

#[test]
fn with_the_tab_chosen_the_key_pressed_on_the_board_itself_goes_back_to_where_you_came_from_and_says_so_when_there_is_nowhere_to_go() {
    use sw::board::run::say::heard;
    let w = world();
    let root = w.root_str();
    w.patch(|s| { s["workspaces"].as_array_mut().unwrap().push(json!({ "workspace_id": "w9", "label": "b", "cwd": root })); for (pane, tab) in [("w9:p1", "w9:t1"), ("w9:p2", "w9:t2"), ("w9:board", "w9:t3")] { s["panes"].as_array_mut().unwrap().push(json!({ "pane_id": pane, "workspace_id": "w9", "tab_id": tab, "terminal_id": format!("term_{pane}"), "cwd": root, "agent": null })); } });
    let note = || sw::store::files::read_json(&sw::store::files::data_dir().join(format!("board-pane-{}.json", sw::herdr::cli::session_name(&w.socket)))).unwrap_or(Value::Null);
    let focus = || w.state().arr("focus").iter().map(text).collect::<Vec<_>>();
    let press = |pane: &str, tab: &str, keep: bool| { let h = here(pane, tab); set_env(&[("HERDR_ENV", "1"), ("HERDR_PLUGIN_CONTEXT_JSON", &h)]); let x = show(None, keep); set_env(&[]); x };
    let arrive_from = |ctx: &str| { set_env(&[("HERDR_ENV", "1"), ("HERDR_PANE_ID", "w9:board"), ("HERDR_PLUGIN_ID", "herdr-projects"), ("HERDR_PLUGIN_CONTEXT_JSON", ctx)]); arrive(); set_env(&[]); };
    // The board the key opened knows from herdr which pane you came from.
    arrive_from(&here("w9:p1", "w9:t1"));
    assert_eq!(note().s("from"), "w9:p1");
    // Pressed on the board: herdr goes back to that pane's space and tab.
    w.patch(|s| s["focus"] = json!([]));
    assert_eq!(press("w9:board", "w9:t3", false), "w9:board");
    assert_eq!(focus(), vec!["workspace w9", "tab w9:t1"]);
    // Pressed from another pane: to the board, and that pane is where the next press on the board goes back to.
    press("w9:p2", "w9:t2", false);
    assert_eq!(focus()[focus().len() - 2..].to_vec(), vec!["workspace w9", "tab w9:t3"]); assert_eq!(note().s("from"), "w9:p2");
    press("w9:board", "w9:t3", false); assert_eq!(focus()[focus().len() - 2..].to_vec(), vec!["workspace w9", "tab w9:t2"]);
    // A board that loads its new version in the same pane keeps what was written down since.
    arrive_from(&json!({ "focused_pane_id": "w9:p1" }).to_string()); assert_eq!(note().s("from"), "w9:p2");
    // A command that needs the board (approve) stays on it.
    let n = focus().len();
    press("w9:board", "w9:t3", true); assert_eq!(focus()[n..].to_vec(), vec!["workspace w9", "tab w9:t3"]);
    // The pane you came from is closed, or the board was started by hand: the board says that it is the board.
    w.patch(|s| { s["panes"].as_array_mut().unwrap().retain(|p| p.s("pane_id") != "w9:p2"); s["focus"] = json!([]); });
    press("w9:board", "w9:t3", false);
    assert!(focus().is_empty());
    assert_eq!(heard("w9:board", &w.socket).map(|h| h.0), Some("This is the board already. There is no pane to go back to.".to_string()));
    std::fs::remove_file(sw::store::files::data_dir().join(format!("board-pane-{}.json", sw::herdr::cli::session_name(&w.socket)))).unwrap();
    set_env(&[("HERDR_ENV", "1"), ("HERDR_PANE_ID", "w9:board")]); arrive(); set_env(&[]); assert!(note().at("from").is_null());
    press("w9:board", "w9:t3", false); assert!(heard("w9:board", &w.socket).is_some());
}
