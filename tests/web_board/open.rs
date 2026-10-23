//! Open this task's agent when its space was closed: the space comes back as a new pane, and the page goes there only
//! once the web UI (Ovis) shows that pane (its page drops a link to a pane it does not know yet); when it never does,
//! or the space cannot be reopened, the page stays and the board's message says why.
use super::make_f;
use crate::common::*;
use serde_json::{json, Value};
use std::cell::Cell;
use sw::json::*;
use sw::webui::attach::far::WebUi;
use sw::webui::pcs::Pc;

/// The web UI as the front door asks it: this machine's PC, which shows the task's pane from its `from`th fresh asking
/// on (None: never), each fresh asking counted.
struct Ui { id: String, from: Option<usize>, asked: Cell<usize> }
impl WebUi for Ui {
    fn pcs(&self) -> Option<Vec<Pc>> { Some(vec![self.local(false)]) }
    fn pcs_now(&self) -> Option<Vec<Pc>> { self.asked.set(self.asked.get() + 1); Some(vec![self.local(self.from.is_some_and(|n| self.asked.get() >= n))]) }
    fn post(&self, _: &str, _: &Value) -> Option<(u16, Value)> { None }
}
impl Ui {
    fn local(&self, shows: bool) -> Pc {
        let mut panes = vec!["w1:p1".to_string()];
        if shows { panes.push(task(&self.id).at("link").t("pane")); }
        Pc { id: "local".into(), name: "home".into(), local: true, connected: true, state: "connected".into(), panes: Some(panes), ..Default::default() }
    }
}
fn ui(id: &str, from: Option<usize>) -> Ui { Ui { id: id.into(), from, asked: Cell::new(0) } }
/// The button pressed, with the web UI's PCs as the front door last kept them (`kept_shows`: with the task's pane).
fn press(ui: &Ui, kept_shows: bool) -> Value { sw::webui::web::call_with("t", "press", json!({ "id": "open", "role": "drive" }), "", Some(vec![ui.local(kept_shows)]), Some(ui)).unwrap() }
/// The task's space closed, as approving or archiving leaves it: its pane until then.
fn close(id: &str) -> String {
    let t = task(id);
    sw::herdr::Herdr::new(Some(t.at("link").t("socket"))).workspace_close(t.at("link").s("workspace")).unwrap();
    t.at("link").t("pane")
}
fn opened(w: &World) -> String {
    let id = make_f(w);
    sw::webui::web::call("t", "pick", json!({ "id": id, "open": true }), "", None).unwrap();
    sw::clock::set(Some(sw::clock::now())); // the wait for the web UI moves this clock, never the real one
    id
}

#[test]
fn a_closed_space_is_reopened_and_the_page_goes_there_once_the_web_ui_shows_its_pane() {
    let w = world();
    let id = opened(&w);
    // its space is open and the web UI already shows its pane: the page goes at once, nothing asked again
    let open = ui(&id, Some(1));
    let s = press(&open, true);
    assert_eq!((s.at("go").i("n"), s.at("go").s("pane")), (1, task(&id).at("link").s("pane")));
    assert_eq!(open.asked.get(), 0);
    // (and the space's name, for the page to say where it did not go: talk.js)
    assert_eq!(s.at("go").s("space"), sw::flow::sidebar::space_label(&task(&id)));
    // closed: it is made again, and the web UI shows the new pane on its third asking; the page goes there then
    let before = close(&id);
    let slow = ui(&id, Some(3));
    let started = sw::clock::now();
    let s = press(&slow, false);
    let pane = task(&id).at("link").t("pane");
    assert_ne!(pane, before, "its space was made again");
    assert_eq!((s.at("go").i("n"), s.at("go").s("pane"), s.at("go").s("workspace")), (2, pane.as_str(), task(&id).at("link").s("workspace")));
    assert_eq!(slow.asked.get(), 3);
    assert!(sw::clock::now() - started < sw::webui::going::BOUND_MS);
    let _ = w;
}

#[test]
fn when_the_web_ui_never_shows_the_reopened_pane_the_page_stays_and_the_board_says_which_space_is_open() {
    let w = world();
    let id = opened(&w);
    close(&id);
    let never = ui(&id, None);
    let started = sw::clock::now();
    let s = press(&never, false);
    assert!(s.at("go").is_null(), "{}", s.at("go"));
    assert!(sw::clock::now() - started >= sw::webui::going::BOUND_MS && never.asked.get() > 3);
    let msg = s.at("msg").s("text");
    assert!(msg.contains("Make f") && msg.contains("is open in herdr, but the web UI (Ovis) does not show it yet") && msg.contains("open the agent again"), "{msg}");
    assert_eq!(s.at("msg").s("kind"), "yellow");
    // nothing is left waiting: a later look does not send the page off on its own
    let shows = ui(&id, Some(1));
    let s = sw::webui::web::call_with("t", "board", json!({ "role": "drive" }), "", None, Some(&shows)).unwrap();
    assert!(s.at("go").is_null() && shows.asked.get() == 0);
    // pressed again once the web UI shows it: the page goes
    let s = press(&shows, true);
    assert_eq!((s.at("go").i("n"), s.at("go").s("pane")), (1, task(&id).at("link").s("pane")));
    let _ = w;
}

#[test]
fn a_space_that_cannot_be_reopened_says_why_and_the_page_stays() {
    let w = world();
    let id = opened(&w);
    close(&id);
    std::fs::remove_dir_all(task(&id).s("worktree")).unwrap();
    let s = press(&ui(&id, Some(1)), false);
    assert!(s.at("go").is_null(), "{}", s.at("go"));
    assert!(s.at("msg").s("text").contains("Its terminal is gone"), "{}", s.at("msg"));
    let _ = w;
}

#[test]
fn the_page_checks_that_the_web_ui_went_and_comes_back_with_a_word_when_it_did_not() {
    const TALK: &str = include_str!("../../crates/web/src/webui/page/talk.js");
    const BOARD: &str = include_str!("../../crates/web/src/webui/page/board.js");
    // written again a few times while the web UI shows neither the pane nor its space; then put back, and the board says so
    assert!(TALK.contains("if ((now === go.pane && shown(go)) || (now !== go.pane && now !== from) || location.hash) return;"));
    assert!(TALK.contains("if (++tries > 3) { history.replaceState(null, \"\", `${location.pathname}${back}`); return missed?.(); }"));
    assert!(BOARD.contains("goTo(snap.go, () => missed(snap.go))") && BOARD.contains("The web UI (Ovis) did not go to ${go.space ? `“${go.space}”` : \"the task's space\"}"));
}
