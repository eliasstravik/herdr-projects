//! The web board's chat docked on the web UI's own chat: where the chat of whoever the board's chat talks to is (the
//! Coordinator's pane, a task's agent's), found without opening or starting anything; and the page that goes there in
//! place under the board, sends a coordinator's words through the board, and puts the web UI back when the board closes.
use crate::common;
use common::board::{web, web_as};
use common::*;
use serde_json::json;
use sw::json::*;

const DOCK: &str = include_str!("../crates/web/src/webui/page/dock.js");
const DOCK_CSS: &str = include_str!("../crates/web/src/webui/page/dock.css");
const CHAT: &str = include_str!("../crates/web/src/webui/page/chat.js");
const BOARD_JS: &str = include_str!("../crates/web/src/webui/page/board.js");
const DOM: &str = include_str!("../crates/web/src/webui/page/dom.js");
const SHEETS: &str = include_str!("../crates/web/src/webui/page/sheets.js");

fn coord_world() -> World {
    let w = world_with(Opts { repo: true, settings: json!({ "coordinator.agent": "claude" }), ..Default::default() });
    let home = w.base.join("home-coord");
    std::fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", &home) };
    w
}
fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }

#[test]
fn the_dock_is_the_pane_of_whoever_the_chat_talks_to_and_nothing_is_opened_for_it() {
    let w = coord_world();
    let socket = sw::herdr::cli::current_socket();
    w.script(json!({ "coordinator": [{ "said": "Hi." }], "worker": [{ "status": "working" }] }));
    // not started yet: no pane, so the board's own chat stands in
    assert_eq!(web(&w, "dock", json!({})).at("dock"), &json!(null));
    // started: its pane's chat view
    web(&w, "send", json!({ "text": "hello" })); drain();
    let pane = sw::coordinator::state::get(&socket).unwrap_or_default().t("pane");
    assert!(!pane.is_empty());
    let d = web(&w, "dock", json!({}));
    assert_eq!((d.at("dock").t("pane"), d.at("dock").s("view"), d.at("dock").t("workspace")), (pane.clone(), "chat", pane.split(':').next().unwrap().to_string()));
    // a task whose agent is in its pane now: that pane, after the flip
    let id = w.add("Make f");
    delegate(&id);
    web(&w, "pick", json!({ "id": id }));
    assert_eq!(web(&w, "press", json!({ "id": "talk" })).at("chat").at("toAgent"), &json!(true));
    let link = task(&id).at("link").clone();
    let d = web(&w, "dock", json!({}));
    assert_eq!((d.at("dock").t("pane"), d.at("dock").t("workspace")), (link.t("pane"), link.t("workspace")));
    // it only looks: no link for the page to follow, no space opened again, nothing said
    assert_eq!((d.at("go"), d.at("msg")), (&json!(null), &json!(null)));
    // a task nobody has been on: none
    let none = w.add("Later");
    web(&w, "pick", json!({ "id": none }));
    assert_eq!(web(&w, "dock", json!({})).at("dock"), &json!(null));
    // a device that may only watch sees it too (the web UI's own chat keeps its own rules on who may type)
    web(&w, "pick", json!({ "id": id }));
    web(&w, "press", json!({ "id": "talk" }));
    assert_eq!(web_as(&w, "test", "dock", json!({ "role": "watch" })).unwrap().at("dock").t("pane"), pane);
    assert!(sw::webui::door::LOOKS.contains(&"dock") && !sw::webui::door::LOOKS.contains(&"full"));
}

#[test]
fn the_page_docks_that_chat_in_place_and_puts_the_web_ui_back_when_the_board_closes() {
    // asked again whenever who the chat talks to changes; the answer docks it, or (null) the board's own chat
    assert!(BOARD_JS.contains(r#"const out = await call("dock");"#) && BOARD_JS.contains("if (root && chat && n === docking && out.ok) chat.native(out.dock || null);"));
    assert!(CHAT.contains("if (whose !== lastWho) { lastWho = whose; asked = \"\"; }") && CHAT.contains("ask(dock.on() ? whose : JSON.stringify([whose, c.status]));"));
    // in place: the address keeps #board (Back still closes the board), and the web UI follows it as it follows Back
    assert!(DOCK.contains("history.replaceState(history.state, \"\", `${location.pathname}?${q}${location.hash}`);") && DOCK.contains(r#"window.dispatchEvent(new PopStateEvent("popstate", { state: history.state }));"#));
    // the web UI keeps the view of a pane it shows: its own Chat button then; not shown in time, the board's own chat
    assert!(DOCK.contains(r#"const btn = document.querySelector(".view-switch button");"#) && DOCK.contains("wait = setTimeout(() => { wait = 0; if (!mine()) { want = null; set(false); } }, WAIT_MS);"));
    // another pane picked in the web UI's own sidebar undocks it
    assert!(DOCK.contains("if (on && !here(want)) { want = null; shown = \"\"; set(false); }"));
    // closing the board puts the web UI back on the pane it showed, where it still shows the docked one
    assert!(DOCK.contains("if (back && location.search !== homeSearch) { history.replaceState(history.state, \"\", `${location.pathname}${homeSearch}${location.hash}`);"));
    // the board stays the board of the PC the page was on, while the chat docked is on another
    assert!(DOM.contains("const machine = () => { if (home !== null) return home;") && DOCK.contains("setHome(home);") && DOCK.contains("setHome(null);"));
    // the web UI's pane column leaves the board its room, its glass header and tabs step aside, the board under its menus
    for rule in [
        "html.sw-dock .pane-column { box-sizing: border-box; padding-top: var(--sw-dock-top, 0px); padding-left: var(--sw-dock-left, 0px); }",
        "html.sw-dock .app { --header-over: 0px !important; }",
        "html.sw-dock .pane-slot .tab-strip-row, html.sw-dock .pane-slot .tab-strip { display: none !important; }",
        "html.sw-dock .sw-board { z-index: 9; }",
        "html.sw-dock .sw-board .sw-scrim { position: fixed; }",
    ] { assert!(DOCK_CSS.contains(rule), "{rule}"); }
    // Escape in the web UI's own chat is the web UI's
    assert!(SHEETS.contains(r#"if (e.key === "Escape" && !theirs(e))"#) && BOARD_JS.contains(r#"(e) => !!e.target?.closest && !e.target.closest(".sw-board") && e.target !== document.body);"#));
}

#[test]
fn a_coordinators_words_in_the_web_uis_box_go_through_the_board_and_tab_stays_the_web_uis_own() {
    // enter (as the web UI's Settings say) and its Send, before the web UI's own handlers; tab is the web UI's own
    assert!(DOCK.contains(r#"import { sendsOn } from "./compose.js";"#) && DOCK.contains("if (!sendsOn(e) || !box?.matches?.(BOX) || !box.closest(COMPOSER) || !box.value.trim()) return;"));
    assert!(!DOCK.contains("\"Tab\""));
    // the web UI's own menu (its `/` commands, its `@` files) open: Enter picks from it, as it does there
    assert!(DOCK.contains("    if (menuOpen(box)) return;\n    const r = route();"));
    assert!(DOCK.contains(r#"const menuOpen = (box) => box.getAttribute("aria-expanded") === "true" || !!box.closest(COMPOSER)?.querySelector(".composer-menu");"#));
    // a file still on its way: nothing goes (its path is not in the words yet)
    assert!(DOCK.contains(r#"const uploading = (box) => !!box.closest(COMPOSER)?.querySelector(".composer-attachment.is-uploading");"#));
    assert!(DOCK.matches("    if (!uploading(box)) sendVia(box, r);").count() == 2 && DOCK.contains("if (sending || uploading(box)) return;"));
    // the files the words carried go with them: their tiles let go by the web UI's own remove, once the board took them
    assert!(DOCK.contains(r#"tiles = [...(box.closest(COMPOSER)?.querySelectorAll(".composer-attachment.is-ready") || [])];"#) && DOCK.contains(r#"for (const t of tiles) if (t.isConnected) t.querySelector("button")?.click();"#));
    assert!(DOCK.contains("    if (!r) return;") && DOCK.contains(r#"document.addEventListener("keydown", onKey, true);"#) && DOCK.contains(r#"document.addEventListener("click", onSend, true);"#));
    // the words leave the web UI's box only once the board took them, as its framework reads an input event
    assert!(DOCK.contains("if (box.value === text) setBox(box, \"\"); // kept: the board did not take them, so they stay") && DOCK.contains(r#"box.dispatchEvent(new Event("input", { bubbles: true }));"#));
    // no Send + start over the web UI's box: the coordinator decides itself what to start
    assert!(!DOCK.contains("Send + start") && CHAT.contains("? { send: (text) => onSend(text, false, {"));
    // a coordinator's (the Coordinator, a machine's or a repo's); a task's agent's go as the web UI sends them
    assert!(CHAT.contains("route: () => (snapNow?.chat && now && (!now.reply || coordHead) && snapNow.role === \"drive\""));
    // the web UI's box with a phone's keyboard up: the board steps aside to its bar (with the chat shown: hidden, it
    // stays hidden, web_keyboard.rs)
    assert!(DOCK.contains(r#"now = on && !!box?.matches?.(BOX) && !!box.closest(COMPOSER) && html.hasAttribute("data-keyboard");"#) && CHAT.contains(r#"board.classList.toggle("sw-web-typing", stepped);"#));
}
