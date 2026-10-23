//! What the old board-needs tests checked that tests/board_needs.rs does not: with colour off the mark alone tells an
//! answer, a yes or no, a sign-in and something that went wrong apart; and a task a limit stopped is read as one that
//! needs you, its row saying the plain line; and what a task that needs you waits for is in its mark's colour.
use serde_json::{json, Value};
use sw::board::theme::Theme;
use sw::json::*;
use sw::shown::states::standing;

fn with(more: Value) -> Obj { let mut b = json!({ "id": "proj/1", "title": "Plan v1", "agent": "claude", "app": "claude", "at": sw::clock::now(), "owner": "me" }); for (k, v) in more.as_object().unwrap() { b[k] = v.clone(); } obj(b) }
fn asks(kind: &str) -> Obj { let mut o = with(json!({ "state": "blocked", "stage": "asks", "msg": "What one job must v1 nail?" })); o.put("kind", kind); o }

#[test]
fn with_colour_off_the_mark_alone_tells_them_apart() {
    let plain = Theme::default_dark(true);
    let broke = with(json!({ "state": "error", "stage": "nosummary", "msg": "Stopped without a summary" }));
    let marks: Vec<String> = [asks("question"), asks("permission"), asks("auth"), broke].iter().map(|r| sw::board::view::fields::mark(&plain, r).text()).collect();
    assert_eq!(marks, vec!["?", "!", "@", "✗"]);
}

#[test]
fn a_task_a_limit_stopped_is_read_as_one_that_needs_you_and_its_row_says_the_plain_line() {
    let msg = "stopped at its 2h limit";
    let t = with(json!({ "stage": "stopped", "msg": msg, "limitStop": { "what": "time", "limit": "2h", "role": "agent", "used": 8400000, "at": 1 } }));
    let shown = sw::state::show(&t, &Value::Null, sw::clock::now());
    assert_eq!((shown.state, shown.msg.as_str()), ("blocked", msg));
    let mut row = t.clone(); row.put("state", shown.state); row.put("msg", shown.msg.clone()); row.put("raw", msg); row.put("link", json!({ "pane": "p1" }));
    assert_eq!(standing(&row), "needs you");
    assert_eq!(sw::shown::row_says::sentence(&row).text(), msg);
}

#[test]
fn what_a_task_that_needs_you_waits_for_is_in_its_marks_colour_on_the_board_and_on_the_web_page() {
    let t = Theme::default_dark(false);
    let mut bare = asks("question"); bare.put("msg", "");
    let login = with(json!({ "state": "blocked", "kind": "auth", "stage": "failed", "msg": "", "failure": { "auth": true, "title": "Needs a login", "detail": "Not logged in", "next": "Sign in to the agent." } }));
    for r in [asks("question"), asks("permission"), asks("auth"), bare, login] {
        let (said, ink) = (sw::shown::row_says::sentence(&r), sw::board::view::fields::mark(&t, &r).0[0].c);
        assert_eq!(said.0.iter().map(|p| p.c).collect::<Vec<_>>(), vec![ink], "{}", said.text());
        assert_eq!(sw::webui::ink::pieces(&said)[0].s("c"), "red");
    }
    // anything else a row says keeps its quiet grey
    let broke = with(json!({ "state": "error", "stage": "nosummary", "msg": "Stopped without a summary" }));
    assert_eq!(sw::shown::row_says::sentence(&broke).0[0].c, sw::shown::ink::Ink::Soft);
}
