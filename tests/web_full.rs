//! The web board's chat full screen: the web UI's own chat view of whoever the chat talks to (the Coordinator, or the
//! selected task's agent), which the page goes to, with a bar over it that flips between the two and goes back; the
//! board's own chat full screen where the web UI cannot show them. And a send that keeps the phone's keyboard up.
use crate::common;
use common::board::{web, web_as};
use common::*;
use serde_json::json;
use sw::json::*;

const TALK: &str = include_str!("../crates/web/src/webui/page/talk.js");
const TALK_CSS: &str = include_str!("../crates/web/src/webui/page/talk.css");
const BOARD_JS: &str = include_str!("../crates/web/src/webui/page/board.js");
const START: &str = include_str!("../crates/web/src/webui/page/start.js");

fn coord_world() -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }

#[test]
fn full_screen_links_to_the_web_uis_chat_view_of_the_coordinator_or_the_tasks_agent_where_it_can() {
    let w = coord_world();
    let socket = sw::herdr::cli::current_socket();
    w.script(json!({ "coordinator": [{ "said": "Hi." }], "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }], "reviewer": [{ "verdict": { "meets": true, "text": "Good." } }] }));
    // not started yet: nobody's pane to show, so the board's own chat goes full screen
    let s = web(&w, "full", json!({}));
    assert_eq!((s.at("full"), s.at("go")), (&json!(false), &json!(null)));
    // started: the web UI's own chat view of its pane
    web(&w, "send", json!({ "text": "hello" })); drain();
    let s = web(&w, "full", json!({}));
    let pane = sw::coordinator::state::get(&socket).unwrap_or_default().t("pane");
    assert!(!pane.is_empty());
    assert_eq!((s.at("full"), s.at("go").i("n"), s.at("go").t("pane"), s.at("go").s("view")), (&json!(true), 1, pane.clone(), "chat"));
    assert_eq!(s.at("go").t("workspace"), pane.split(':').next().unwrap());
    // the selected task's agent (its work ready to approve), after the flip: Open agent's link (its space reopened first)
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    web(&w, "pick", json!({ "id": id }));
    let s = web(&w, "press", json!({ "id": "talk" }));
    assert_eq!(s.at("chat").at("toAgent"), &json!(true));
    let s = web(&w, "full", json!({}));
    let link = task(&id).at("link").clone();
    assert_eq!((s.at("full"), s.at("go").i("n"), s.at("go").t("pane"), s.at("go").t("workspace")), (&json!(true), 2, link.t("pane"), link.t("workspace")));
    // a task nobody has been on: no pane, and nothing said about it (the board's own chat goes full screen)
    let none = w.add("Later");
    web(&w, "pick", json!({ "id": none }));
    let s = web(&w, "full", json!({}));
    assert_eq!((s.at("full"), s.at("go").i("n"), s.at("msg")), (&json!(false), 2, &json!(null)));
    // back to the Coordinator; a device that may only watch gets the board's own full screen
    web(&w, "pick", json!({ "id": id }));
    let s = web(&w, "press", json!({ "id": "talk" }));
    assert_eq!(s.at("chat").at("toAgent"), &json!(false));
    assert_eq!(web_as(&w, "test", "full", json!({ "role": "watch" })).unwrap().at("full"), &json!(false));
    assert_eq!(web(&w, "full", json!({})).at("go").i("n"), 3);
    // a watching device may ask (it only looks)
    assert!(sw::webui::door::LOOKS.contains(&"full"));
}

#[test]
fn the_page_goes_full_screen_into_the_web_uis_chat_view_with_a_bar_that_flips_and_goes_back() {
    // the full screen button asks the board; its next link is the full screen's, else the board's own chat goes full
    assert!(BOARD_JS.contains(r#"const out = await call("full");"#) && BOARD_JS.contains("if (root && !(out.ok && out.full)) { fullAsked = false; chat?.full(true); }"));
    assert!(BOARD_JS.contains("return full ? goFull(snap.go, snap) : goTo(snap.go);"));
    // the bar shows only on the pane it went to, while the board is closed; it never takes the focus from the web UI's box
    assert!(TALK.contains(r#"q.get("pane") === x.pane"#) && TALK.contains("const x = kept(), on = !open && here(x);"));
    assert!(TALK.contains(r#"bar.addEventListener("pointerdown", (e) => e.preventDefault());"#));
    // a flip presses the board's own key, asks again, and shows the other one's view in place of this one (Back still
    // goes to the board); one the web UI cannot show is the board's, full screen
    assert!(TALK.contains(r#"let out = await post("press", { id: "talk" });"#) && TALK.contains(r#"if (out.ok) out = await post("full");"#));
    assert!(TALK.contains("return goFull(out.go, out, true);") && TALK.contains("keep(null); sync(); onBoard(true);"));
    assert!(TALK.contains("if (x && x.depth === history.length && here(x)) history.back(); else onBoard(false);"));
    assert!(START.contains("fullBar((full) => show(true, full));"));
    // under the web UI's drawer, and its first message is not under the bar
    assert!(TALK_CSS.contains("z-index: 14;") && TALK_CSS.contains("html.sw-full-on .pane-frame.is-current .chat-virtual-header { padding-top:"));
}
