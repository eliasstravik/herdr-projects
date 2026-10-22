//! On the web board a tap selects and opening is a step of its own: the board keeps the task closed on a pick that
//! does not open it, the space key flips the chat to the selected task's agent and back, and the page asks to open only
//! from the selected row's Open button (or, wide, with a task already open beside the list). Whose tasks show is the
//! filter row's first item, whose tap picks another from the same list as the sidebar's Boards.
use crate::common;
use common::board::web;
use common::*;
use crate::far::Far;
use serde_json::json;
use sw::json::*;

const LIST: &str = include_str!("../crates/web/src/webui/page/list.js");
const BOARD: &str = include_str!("../crates/web/src/webui/page/board.js");
const SHEETS: &str = include_str!("../crates/web/src/webui/page/sheets.js");

#[test]
fn a_pick_that_does_not_open_selects_and_space_flips_the_chat_to_that_tasks_agent_and_back() {
    let w = world();
    let one = w.add("Write notes");
    let two = w.add("Read notes");
    let s = web(&w, "pick", json!({ "id": one, "open": false }));
    assert_eq!((s.at("list").s("selected"), s.at("panel").is_null(), s.at("flags").at("open")), (one.as_str(), true, &json!(false)), "selected, not opened");
    // space, and space again: the selected task's agent, then the Coordinator (here: a new task)
    let s = web(&w, "press", json!({ "id": "talk" }));
    assert_eq!((s.at("chat").at("toAgent"), s.at("panel").is_null()), (&json!(true), true));
    assert_eq!(web(&w, "press", json!({ "id": "talk" })).at("chat").at("toAgent"), &json!(false));
    // another row selected: still closed; opening is its own call
    assert!(web(&w, "pick", json!({ "id": two, "open": false })).at("panel").is_null());
    assert_eq!(web(&w, "pick", json!({ "id": two, "open": true })).at("panel").s("id"), two);
}

#[test]
fn the_page_selects_on_a_tap_and_opens_from_the_selected_rows_button() {
    // a tap on a row selects it, never opens it; the selected row has its Open button; Enter there opens it
    assert!(LIST.contains("onclick: () => { if (!selected) onPick(r.id); }"));
    assert!(LIST.contains("if (selected && e.key === \"Enter\") onOpen(r.id); else onPick(r.id);"));
    assert!(LIST.contains("selected && !opened ? openEl(r, onOpen) : null") && LIST.contains("r.automation ? \"Open automation\" : \"Open task\""));
    // the board: a pick opens only where a task is open beside the list already (wide); Open opens
    assert!(BOARD.contains("onPick: (id) => call(\"pick\", { id, open: !!snap.panel && wide() }), onOpen: (id) => call(\"pick\", { id, open: true })"));
    assert!(BOARD.contains("window.matchMedia?.(\"(min-width: 900px)\").matches"));
}

#[test]
fn whose_tasks_show_is_the_filter_rows_first_item_and_a_tap_on_it_picks() {
    // whose board stands first on the filter row (as on the terminal board), never a chip of its own; a tap on it lists
    // every board as the sidebar's Boards section does, the one shown checked
    assert!(BOARD.contains(r#"onclick: () => { picking = true; draw(); } }, h("span.sw-show-name", m.name), icon("down"))"#));
    assert!(!BOARD.contains("Show only this machine's tasks") && !BOARD.contains("head.machines"), "no chip for whose tasks show");
    assert!(!BOARD.contains("() => press(\"scope\")"), "nothing goes round the machines by itself");
    // the filter row is one row everywhere: it never wraps; with a mouse the wheel scrolls it sideways
    let css = include_str!("../crates/web/src/webui/page/board.css");
    let rules: String = css.lines().filter(|l| l.contains(".sw-shows")).collect();
    assert!(!rules.contains("flex-wrap: wrap") && rules.contains("overflow-x: auto"), "{rules}");
    assert!(BOARD.contains("el.scrollLeft += e.deltaY;"));
    assert!(SHEETS.contains("const out = await get(\"boards\");") && SHEETS.contains("\"aria-current\": b.scope === scope ? \"true\" : null"));
    let w = world_with(Opts { repo: true, settings: json!({ "board.machines": "vm-a" }), ..Default::default() });
    let far = Far::new(&w.base);
    far.tasks("vm-a", "0.2.0", json!([]));
    // every machine: All machines, plain; one machine: its name, lit as the filter shown is
    let s = web(&w, "scope", json!({ "to": "*" }));
    assert_eq!((s.at("list").at("shows").at("machine").s("name"), s.at("list").at("shows").at("machine").at("lit")), ("All machines", &json!(false)), "{}", s.at("list").at("shows"));
    assert!(!s.at("list").at("head").s("machines").is_empty());
    let s = web(&w, "scope", json!({ "to": "" }));
    assert_eq!((s.at("flags").s("scope"), s.at("list").at("head").s("machines"), s.at("list").at("shows").at("machine").at("lit")), ("", "", &json!(true)));
}
