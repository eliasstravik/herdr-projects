//! The board key's two ways to open the board (`board.open`): a tab unless you choose herdr's popup. The popup it
//! opens, closes when run again and moves to the tab that shows; a board in a tab running beside it, each with its own
//! note, selection and words; herdr refusing the popup, and the board in a tab saying why.
use crate::common;
use common::board::{open_with, Board};
use common::herdr::FakeHerdr;
use common::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;
use sw::board::keys::key_name;
use sw::board::place::{arrive, context, depart, in_popup, own_name, own_pane, show};
use sw::herdr::cli::{Fake, HerdrError, Ran};
use sw::json::*;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn set_env(pairs: &[(&str, &str)]) { for k in ["HERDR_ENV", "HERDR_PANE_ID", "HERDR_PLUGIN_ID", "HERDR_PLUGIN_CONTEXT_JSON"] { unsafe { std::env::remove_var(k) } } for (k, v) in pairs { unsafe { std::env::set_var(k, v) } } }
/// Where the human is when the key is pressed: the pane that has the keyboard, and its tab (what herdr tells a plugin).
fn here(pane: &str, tab: &str) -> String { json!({ "focused_pane_id": pane, "tab_id": tab }).to_string() }
/// herdr runs the board key's action: `here` is where you are.
fn press_key(pane: &str, tab: &str, keep: bool) -> String { let h = here(pane, tab); set_env(&[("HERDR_ENV", "1"), ("HERDR_PLUGIN_CONTEXT_JSON", &h)]); let x = show(None, keep); set_env(&[]); x }
/// A board starting in herdr's popup over `tab`, or in a pane: it writes down where it is.
fn arrive_popup(tab: &str) { let h = here("", tab); set_env(&[("HERDR_ENV", "1"), ("HERDR_PLUGIN_ID", "herdr-projects"), ("HERDR_PLUGIN_CONTEXT_JSON", &h)]); arrive(); set_env(&[]); }
fn arrive_pane(pane: &str) { set_env(&[("HERDR_ENV", "1"), ("HERDR_PANE_ID", pane)]); arrive(); set_env(&[]); }
fn note(name: &str) -> Value { sw::store::files::read_json(&note_file(name)).unwrap_or(Value::Null) }
fn note_file(name: &str) -> std::path::PathBuf { sw::store::files::data_dir().join(format!("board-{name}-{}.json", sw::herdr::cli::session_name(&sw::herdr::cli::current_socket()))) }
/// A space with a shell in it, and a second tab: where the human is when the key is pressed.
fn space(w: &World) { let root = w.root_str(); w.patch(|s| { s["workspaces"].as_array_mut().unwrap().push(json!({ "workspace_id": "w9", "label": "b", "cwd": root })); for (pane, tab) in [("w9:p1", "w9:t1"), ("w9:p2", "w9:t2")] { s["panes"].as_array_mut().unwrap().push(json!({ "pane_id": pane, "workspace_id": "w9", "tab_id": tab, "terminal_id": format!("term_{pane}"), "cwd": root, "agent": null })); } s["shows"] = json!("w9:t1"); }); }

/// herdr's popup, on top of the test's fake herdr: one at a time, over the tab that shows (`shows`); `busy` refuses it.
/// The board in it is a process of its own (`board`), which ends when the popup closes.
struct Pop { inner: Rc<FakeHerdr>, board: RefCell<Option<std::process::Child>> }
fn answer(code: i32, v: Value) -> Ran { if code == 0 { Ran { status: 0, stdout: format!("{}\n", json!({ "id": "fake", "result": v })), stderr: String::new() } } else { Ran { status: 1, stdout: String::new(), stderr: format!("{}\n", json!({ "id": "fake", "error": v })) } } }
impl Fake for Pop {
    fn run(&self, args: &[String], input: &str) -> Ran {
        if !(args.len() > 2 && args[0] == "plugin" && args[1] == "pane" && args.iter().any(|a| a == "popup")) { return self.inner.run(args, input); }
        let mut st = self.inner.st.borrow_mut();
        let flag = |n: &str| args.iter().position(|a| a == n).and_then(|i| args.get(i + 1)).cloned().unwrap_or_default();
        if st.b("busy") { let b = st.t("busy"); return answer(1, json!({ "code": b, "message": if b == "ui_busy" { "busy" } else { "herdr said no" } })); }
        let tab = st.t("shows");
        st["boardPlacement"] = json!("popup"); st["popup"] = json!({ "tab": tab, "width": flag("--width"), "height": flag("--height") });
        st["focus"].as_array_mut().unwrap().push(json!(format!("popup over {tab}")));
        answer(0, json!({ "type": "ok" }))
    }
    fn kids(&self, pid: i64) -> Vec<i64> { self.inner.kids(pid) }
    fn kill(&self, pid: i64) { self.inner.kill(pid) }
    fn ask(&self, method: &str, _params: &Value) -> Result<Value, HerdrError> {
        let mut st = self.inner.st.borrow_mut();
        if method != "popup.close" { return Err(HerdrError { code: "no_server".into(), message: "no herdr".into() }); }
        if st.at("popup").is_null() { return Err(HerdrError { code: "popup_not_open".into(), message: "none".into() }); }
        st["popup"] = Value::Null; st["focus"].as_array_mut().unwrap().push(json!("popup closed"));
        if let Some(mut c) = self.board.borrow_mut().take() { let _ = c.kill(); let _ = c.wait(); }
        Ok(json!({}))
    }
}
fn popup_world(settings: Value) -> (World, Rc<Pop>) {
    let w = world_with(Opts { repo: true, settings, ..Default::default() });
    let p = Rc::new(Pop { inner: w.fake.clone(), board: RefCell::new(None) });
    sw::herdr::cli::set_fake(Some(p.clone()));
    (w, p)
}
/// The board in the popup runs: a process of its own, written down as the popup's.
fn board_runs(p: &Pop) { let c = std::process::Command::new("sleep").arg("60").spawn().unwrap(); let mut n = note("popup"); n["pid"] = json!(c.id()); sw::store::files::write_json(&note_file("popup"), &n).unwrap(); *p.board.borrow_mut() = Some(c); }
fn up(w: &World) -> Value { w.state().at("popup").clone() }
fn focus(w: &World) -> Vec<String> { w.state().arr("focus").iter().map(text).collect() }
fn view(name: &str) -> Obj { sw::store::files::read_obj(&sw::store::files::data_dir().join(name)).unwrap_or_default() }

#[test]
fn board_open_is_a_setting_like_the_others_a_tab_unless_you_choose_the_popup() {
    let _w = world();
    let setting = || sw::store::settings::lookup::setting("board.open", None);
    let main = |a: &[&str]| sw::cli::main(a.iter().map(|x| x.to_string()).collect());
    assert_eq!(setting(), json!("tab"));
    assert_eq!(main(&["set", "board.open", "popup"]), 0); assert_eq!(setting(), json!("popup"));
    assert_ne!(main(&["set", "board.open", "window"]), 0); assert_eq!(setting(), json!("popup"));
    assert_eq!(main(&["unset", "board.open"]), 0); assert_eq!(setting(), json!("tab"));
    // herdr's popup is no pane: its program gets the plugin's id and no pane id. Anything else is a pane.
    set_env(&[("HERDR_ENV", "1"), ("HERDR_PLUGIN_ID", "herdr-projects")]);
    assert!(in_popup()); assert_eq!(own_pane(), "", "a popup is never gone back to"); assert_eq!(own_name(), "popup");
    set_env(&[("HERDR_ENV", "1"), ("HERDR_PLUGIN_ID", "herdr-projects"), ("HERDR_PANE_ID", "w9:p7")]);
    assert!(!in_popup()); assert_eq!(own_pane(), "w9:p7"); assert_eq!(own_name(), "w9:p7");
    set_env(&[]); assert_eq!(own_name(), "");
    set_env(&[("HERDR_PLUGIN_CONTEXT_JSON", &here("w9:p1", "w9:t1"))]); assert_eq!(context(), ("w9:p1".to_string(), "w9:t1".to_string()));
    set_env(&[("HERDR_PLUGIN_CONTEXT_JSON", "{")]); assert_eq!(context(), (String::new(), String::new()));
    set_env(&[]);
}

#[test]
fn with_the_popup_chosen_the_key_opens_the_board_in_herdrs_popup_closes_it_when_run_again_and_takes_one_left_behind_over_another_tab_with_it() {
    let (w, p) = popup_world(json!({ "board.open": "popup" }));
    space(&w);
    // None is open: herdr's popup opens, at the board's size. No pane and no tab is made for it.
    let panes = w.state().arr("panes").len();
    assert_eq!(press_key("w9:p1", "w9:t1", false), "popup");
    assert_eq!(up(&w), json!({ "tab": "w9:t1", "width": "90%", "height": "90%" })); assert_eq!(w.state().s("boardPlacement"), "popup");
    assert_eq!(w.state().arr("panes").len(), panes); assert!(!w.state().arr("tabs").iter().any(|t| t.s("label") == "board"));
    // Run again before its board has even started: that one is closed, and no second is opened over it.
    assert_eq!(note("popup").s("tab"), "w9:t1"); assert_eq!(note("popup").i("pid"), 0);
    assert_eq!(press_key("w9:p1", "w9:t1", false), ""); assert!(up(&w).is_null()); assert!(note("popup").is_null());
    assert_eq!(press_key("w9:p1", "w9:t1", false), "popup"); assert!(!up(&w).is_null());
    arrive_popup("w9:t1"); // its board starts and writes down where it is
    assert_eq!(note("popup"), json!({ "tab": "w9:t1", "pid": std::process::id() }));
    arrive_pane("w9:p2"); assert_eq!(note("popup").i("pid"), std::process::id() as i64, "a board in a pane writes no popup note");
    board_runs(&p);
    // A command that needs the board (approve the task in this pane) leaves the one that shows open.
    assert_eq!(press_key("w9:p1", "w9:t1", true), "popup"); assert!(!up(&w).is_null()); assert_eq!(focus(&w).last().unwrap(), "popup over w9:t1");
    // Run again while it shows: closed. (herdr gives the popup every key, so the key itself is the board's to see:
    // board_stay_keys.rs. This is the action run another way.)
    assert_eq!(press_key("w9:p1", "w9:t1", false), ""); assert!(up(&w).is_null()); assert_eq!(focus(&w).last().unwrap(), "popup closed");
    // Still there over a tab that no longer shows (its board closes it within a moment by itself): closed there,
    // opened here. The key is never dead.
    press_key("w9:p1", "w9:t1", false); arrive_popup("w9:t1"); board_runs(&p);
    w.patch(|s| s["shows"] = json!("w9:t2"));
    assert_eq!(press_key("w9:p2", "w9:t2", false), "popup");
    assert_eq!(up(&w).s("tab"), "w9:t2"); assert_eq!(focus(&w)[focus(&w).len() - 2..].to_vec(), vec!["popup closed", "popup over w9:t2"]);
    // Its board ended without saying so (herdr ends the popup with its program): the note is taken away and a new one opens.
    sw::store::files::write_json(&note_file("popup"), &json!({ "tab": "w9:t2", "pid": 1i64 << 30 })).unwrap(); w.patch(|s| s["popup"] = Value::Null);
    assert_eq!(press_key("w9:p2", "w9:t2", false), "popup"); assert!(!up(&w).is_null()); assert_eq!(note("popup").i("pid"), 0);
    // Only the board that wrote the note takes it away.
    sw::store::files::write_json(&note_file("popup"), &json!({ "tab": "w9:t2", "pid": 1 })).unwrap(); depart(); assert!(!note("popup").is_null());
    arrive_popup("w9:t2"); depart(); assert!(note("popup").is_null());
}

#[test]
fn a_board_in_a_tab_keeps_running_beside_the_popup_each_has_its_own_note_and_its_own_selection_and_words() {
    let (w, p) = popup_world(json!({ "board.open": "popup" }));
    space(&w);
    let (a, b) = (w.add("First"), w.add("Second"));
    arrive_pane("w9:p2"); // a board you started yourself in a pane
    let mut tab = open_with(&w.root, 34, false); tab.s.sel = a.clone(); tab.load(); tab.typed("for the tab"); sw::board::run::prefs::save(&tab.s);
    let over = press_key("w9:p1", "w9:t1", false); arrive_popup("w9:t1"); board_runs(&p);
    assert_eq!(over, "popup"); assert_eq!(note("pane").s("pane"), "w9:p2", "the tab board's note is its own"); assert_eq!(note("popup").s("tab"), "w9:t1");
    // The first popup starts from the tab's view, without the words typed there: they are in that board's box.
    let mut pop = open_with(&w.root, 34, true);
    assert_eq!(pop.s.sel, a); assert_eq!(pop.s.value(), "");
    pop.s.sel = b.clone(); pop.load(); pop.typed("for the popup"); sw::board::run::prefs::save(&pop.s);
    assert_eq!(view("view-popup.json").s("draft"), "for the popup");
    assert_eq!(view("view.json").s("draft"), "for the tab"); assert_eq!(view("view.json").s("sel"), a);
    let mut again = open_with(&w.root, 34, true); assert_eq!(again.s.sel, b); assert_eq!(again.s.value(), "for the popup");
    // esc steps back as on every board (the draft is put away), and the last esc closes the popup. The tab board
    // only shows how it closes.
    again.press(&["esc"]); assert!(!again.closed); assert_eq!(again.s.value(), "");
    again.press(&["esc"]); assert!(again.closed);
    tab.press(&["esc", "esc"]); assert!(!tab.closed); assert!(has(&tab.text(), &format!(r"\n {{2}}{} closes the board *$", regex::escape(&key_name("close")))), "{}", tab.text());
    // Closing the popup leaves the tab board's note alone.
    press_key("w9:p1", "w9:t1", false); assert!(up(&w).is_null()); assert_eq!(note("pane").s("pane"), "w9:p2");
    // "Approve the task in this pane" is asked by the board that was shown for it, not by whichever looks first.
    use sw::actions::finish_line::{ask_approve, take_approve};
    ask_approve(&a, &w.socket, &over).unwrap();
    assert_eq!(take_approve(&w.socket, "w9:p2"), "", "the tab board leaves it"); assert_eq!(take_approve(&w.socket, &over), a);
    ask_approve(&a, &w.socket, "").unwrap(); assert_eq!(take_approve(&w.socket, "w9:p2"), a, "one for no board in particular is anybody's");
}

/// What the loader reads for the board in pane `own` (the lines left for it), as the board's loop would ask.
fn load_own(b: &mut Board, own: &str) {
    use sw::board::update::{update, Effect, Event};
    let d = sw::board::data::load(&sw::board::data::Wants { ids: vec![b.s.sel.clone(), b.s.reply_id.clone()], cwd: b.s.ctx.cwd.clone(), own_pane: own.into(), web: false, ..Default::default() });
    let mut fx = update(&mut b.s, Event::Loaded(Box::new(d)));
    while let Some(e) = fx.pop() { if let Effect::Run(j) = e { let d = sw::board::job::run(j); fx.extend(update(&mut b.s, Event::Done(d))); } }
}

#[test]
fn herdr_will_not_open_the_popup_the_board_opens_in_a_tab_and_says_why() {
    use sw::board::run::say::{heard, say};
    let (w, _p) = popup_world(json!({ "board.open": "popup" }));
    space(&w);
    w.patch(|s| s["busy"] = json!("ui_busy"));
    let pane = press_key("w9:p1", "w9:t1", false);
    assert_eq!(w.state().s("boardPlacement"), "tab"); assert!(!pane.is_empty()); assert_ne!(pane, "popup"); assert!(up(&w).is_null());
    // The line is for that board: another one leaves it, and it is said once.
    assert_eq!(heard("w9:p2", &w.socket), None);
    let mut s = open_with(&w.root, 34, false); load_own(&mut s, &pane);
    assert!(s.text().split_whitespace().collect::<Vec<_>>().join(" ").contains("herdr has another popup open, maybe hidden behind another tab, and shows one at a time. Close that one for the board's popup. Until then the board is here, in a tab."), "{}", s.text());
    assert_eq!(heard(&pane, &w.socket), None);
    // That board runs now: the key goes to it and says it again, while herdr still refuses. It never goes "back".
    arrive_pane(&pane);
    assert_eq!(press_key(&pane, "x", false), pane);
    s.s.msg.clear(); load_own(&mut s, &pane); assert!(s.text().contains("herdr has another popup open"), "{}", s.text());
    // Any other refusal is said with herdr's own words.
    w.patch(|s| s["busy"] = json!("boom"));
    press_key("w9:p1", "w9:t1", false);
    s.s.msg.clear(); load_own(&mut s, &pane); assert!(has(&s.text().split_whitespace().collect::<Vec<_>>().join(" "), r"herdr did not open the popup \(herdr said no\)\. Until then"), "{}", s.text());
    // A line nobody took in time is dropped, not shown late.
    say(&pane, "old news", "yellow", &w.socket);
    sw::clock::set(Some(sw::clock::now() + 60000)); assert_eq!(heard(&pane, &w.socket), None); sw::clock::set(None); assert_eq!(heard(&pane, &w.socket), None);
}
