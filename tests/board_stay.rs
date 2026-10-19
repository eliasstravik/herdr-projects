//! Going into an agent never ends the board: in a pane of its own it runs on, keeps its selection on the task you went
//! into whatever that task becomes, and opening the board from anywhere in herdr goes back to the one that runs, or
//! opens one in a tab of its own. Only a board in herdr's popup closes for an agent.
use crate::common;
use common::board::{open, open_with, Board};
use common::*;
use serde_json::json;
use sw::board::job;
use sw::board::keys::Press;
use sw::board::state::{Ctx, State};
use sw::board::update::{list::relist, update, Effect, Event};
use sw::json::*;

fn has(text: &str, re: &str) -> bool { regex::Regex::new(re).unwrap().is_match(text) }
fn focus(w: &World) -> Vec<String> { w.state().arr("focus").iter().map(text).collect() }
fn view_sel(name: &str) -> String { sw::store::files::read_obj(&sw::store::files::data_dir().join(name)).unwrap_or_default().t("sel") }
fn set_env(pairs: &[(&str, &str)]) { for k in ["HERDR_ENV", "HERDR_PANE_ID", "HERDR_PLUGIN_ID", "HERDR_PLUGIN_CONTEXT_JSON"] { unsafe { std::env::remove_var(k) } } for (k, v) in pairs { unsafe { std::env::set_var(k, v) } } }
/// A board opened as `open` does, with the board key it was told.
fn open_keyed(root: &std::path::Path, board_key: &str) -> Board {
    let ctx = Ctx { cwd: root.display().to_string(), can_reload: true, in_herdr: sw::herdr::cli::in_herdr(), board_key: board_key.into(), ..Default::default() };
    let mut s = State::new(ctx, Default::default());
    sw::board::run::prefs::apply(&mut s, &sw::board::run::prefs::read(false));
    s.side = true;
    let mut b = Board { s, rows: 34, cols: 120, closed: false, theme: sw::board::theme::Theme::default_dark(true) };
    b.load(); relist(&mut b.s); b.load();
    b
}
/// A key whose job may hand this terminal to an agent (outside a herdr client): what it attaches to is `went`, and
/// `then` runs while you are in there.
fn press_attaching(b: &mut Board, key: &str, went: &mut Vec<String>, then: &dyn Fn()) {
    let mut fx = update(&mut b.s, Event::Key(Press::chord(key)));
    while let Some(e) = fx.pop() {
        match e {
            Effect::Run(j) => { let d = job::run(j); fx.extend(update(&mut b.s, Event::Done(d))); }
            Effect::Attach { args, .. } => { went.push(args.last().cloned().unwrap_or_default()); then(); }
            Effect::Close => b.closed = true,
            Effect::Save => sw::board::run::prefs::save(&b.s),
            _ => {}
        }
    }
    b.load();
}
fn shown(b: &Board) -> Vec<String> { let mut v: Vec<String> = b.s.rows.iter().filter_map(|r| if let sw::board::state::Row::Rec(x) = r { Some(x.t("id")) } else { None }).collect(); v.sort(); v }

#[test]
fn enter_on_a_task_goes_into_its_agent_and_the_board_stays_as_it_was_from_the_row_from_the_chat() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    set_env(&[("HERDR_ENV", "1")]);
    w.script(json!({ "worker": [{ "status": "working", "screen": "● Editing greet.sh\n" }] }));
    let id = w.add("Add a greeting"); w.add("Another one");
    delegate(&id); settle(&id);
    let pane = task(&id).at("link").t("pane");
    for (way, keys) in [("from its row", vec![]), ("from its row, the chat hidden", vec!["alt+c"]), ("from the chat with its agent", vec!["space"])] {
        let mut s = open_keyed(&w.root, "prefix+w");
        s.s.sel = id.clone(); s.s.filter = "greeting".into(); s.load(); relist(&mut s.s);
        s.press(&keys);
        let before = focus(&w).len();
        s.press(&["enter"]);
        assert!(!s.closed, "the board is not closed: {way}"); assert!(!s.s.leaving, "{way}");
        assert_eq!(focus(&w)[before..].to_vec(), vec![format!("agent {pane}")], "herdr moved to the agent: {way}");
        assert_eq!(s.s.sel, id, "{way}"); assert_eq!(s.s.filter, "greeting", "{way}"); assert_eq!(s.s.data.repo, "proj", "{way}");
        assert!(has(&s.text().split_whitespace().collect::<Vec<_>>().join(" "), r"Opened (PRJ-1|proj/1)'s agent\. prefix\+w brings you back to the board\."), "{way}: {}", s.text());
        assert_eq!(view_sel("view.json"), id, "where you were is saved: {way}");
        // It still takes keys when you are back, and a second enter goes in again.
        s.press(&["enter"]);
        assert!(!s.closed, "{way}"); assert_eq!(focus(&w).len(), before + 2, "{way}");
        if way == "from its row" { s.press(&["esc"]); assert_eq!(s.s.filter, "", "{way}"); s.press(&["down"]); assert_ne!(s.s.sel, id, "{way}"); }
    }
    // While you are in the agent its task changes where it stands (it stops): the selection is still on that task when
    // you come back, not on the row that took its place, and enter goes into the same agent again.
    {
        let mut s = open(&w.root, 34); s.s.sel = id.clone(); s.s.filter.clear(); s.load(); relist(&mut s.s);
        s.press(&["enter"]); assert!(!s.closed);
        sw::actions::talk::stop(&id).unwrap(); assert_eq!(task(&id).s("stage"), "stopped");
        s.load(); s.load();
        assert_eq!(s.s.sel, id, "the selection did not move to its neighbour"); assert_eq!(view_sel("view.json"), id);
        let before = focus(&w).len();
        s.press(&["enter"]);
        assert!(!s.text().contains("No agent has been on this yet"), "{}", s.text()); assert_eq!(focus(&w)[before..].to_vec(), vec![format!("agent {pane}")]);
        s.load(); assert_eq!(s.s.sel, id);
        // Back and moving on your own: the rows follow the keys again.
        s.press(&["down"]); if s.s.sel == id { s.press(&["up"]); }
        assert_eq!(s.s.sel, "proj/2");
    }
    // The same under a filter the task stops matching while you are away ("working", and its agent stops): its row
    // stays, selected, until you move on yourself.
    {
        w.script(json!({ "worker": [{ "status": "working", "screen": "● Editing\n" }] }));
        let other = w.add("Say goodbye"); let near = w.add("A working neighbour");
        delegate(&other); settle(&other); assert_eq!(task(&other).s("stage"), "working");
        let there = task(&other).at("link").t("pane");
        let mut s = open(&w.root, 34); s.s.filter = "working".into(); s.s.sel = other.clone(); s.load(); relist(&mut s.s);
        let mut both = vec![other.clone(), near.clone()]; both.sort();
        assert_eq!(shown(&s), both);
        s.press(&["enter"]); assert!(!s.closed);
        sw::actions::talk::stop(&other).unwrap(); s.load(); s.load();
        assert_eq!(s.s.sel, other, "the selection did not move to the task that still matches"); assert_eq!(s.s.filter, "working");
        assert_eq!(view_sel("view.json"), other);
        let before = focus(&w).len();
        s.press(&["enter"]);
        assert!(!s.text().contains("No agent has been on this yet")); assert_eq!(focus(&w)[before..].to_vec(), vec![format!("agent {there}")]);
        // Moving on yourself lets the row go: it does not match what you are looking for.
        s.press(&["down"]); if s.s.sel == other { s.press(&["up"]); }
        s.load(); assert_eq!(s.s.sel, near); assert_eq!(shown(&s), vec![near.clone()]);
        s.s.filter.clear(); sw::board::run::prefs::save(&s.s);
    }
    // Outside a herdr client (plain SSH) the same terminal attaches to the agent and the board comes back when that
    // ends. It is the same there: the task stopped while you were in it, under a filter it no longer matches.
    {
        w.script(json!({ "worker": [{ "status": "working", "screen": "● Editing\n" }] }));
        let other = w.add("Say hello again"); w.add("Another working neighbour");
        delegate(&other); settle(&other);
        let there = task(&other).at("link").t("pane");
        set_env(&[]);
        let mut s = open(&w.root, 34); s.s.filter = "working".into(); s.s.sel = other.clone(); s.load(); relist(&mut s.s);
        let mut went = vec![];
        let o = other.clone();
        press_attaching(&mut s, "enter", &mut went, &|| { sw::actions::talk::stop(&o).unwrap(); });
        assert_eq!(went, vec![there.clone()]); assert!(!s.closed); assert_eq!(task(&other).s("stage"), "stopped");
        assert_eq!(s.s.sel, other, "back on the task you went into"); s.load(); s.load(); assert_eq!(s.s.sel, other);
        press_attaching(&mut s, "enter", &mut went, &|| {}); assert_eq!(went, vec![there.clone(), there.clone()]);
        s.press(&["down"]); if s.s.sel == other { s.press(&["up"]); }
        s.load(); assert_ne!(s.s.sel, other); assert!(!shown(&s).contains(&other));
        s.s.filter.clear(); sw::board::run::prefs::save(&s.s);
        set_env(&[("HERDR_ENV", "1")]);
    }
    // An agent that has exited leaves a bare pane: herdr goes to its space and tab, and the board stays too.
    w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"] = serde_json::Value::Null; } } });
    let mut s = open(&w.root, 34); s.s.sel = id.clone(); s.load(); relist(&mut s.s);
    let before = focus(&w).len();
    s.press(&["enter"]);
    assert!(!s.closed); assert!(has(&focus(&w)[before..].join(" "), r"^workspace \S+"), "{:?} {}", focus(&w), s.text()); assert!(s.text().contains("The board stays here."), "{}", s.text());
    // Only a board someone opened as herdr's popup closes for an agent: a popup lies over every pane, the agent's too.
    let mut p = open_with(&w.root, 34, true); p.s.sel = id.clone(); p.load(); relist(&mut p.s); p.press(&["enter"]);
    assert!(p.closed);
    assert_eq!(view_sel("view-popup.json"), id);
    set_env(&[]);
}

#[test]
fn opening_the_board_from_anywhere_in_herdr_goes_back_to_the_one_that_is_running_and_opens_it_in_a_tab_of_its_own_never_the_popup_when_there_is_none() {
    use sw::board::place::{arrive, depart, go_back, in_popup, own_pane};
    let w = world();
    let root = w.root_str();
    w.patch(|s| { s["workspaces"].as_array_mut().unwrap().push(json!({ "workspace_id": "w9", "label": "b", "cwd": root })); s["panes"].as_array_mut().unwrap().push(json!({ "pane_id": "w9:p1", "workspace_id": "w9", "tab_id": "w9:t1", "terminal_id": "term_board", "cwd": root, "agent": null })); });
    set_env(&[("HERDR_ENV", "1"), ("HERDR_PANE_ID", "w9:p1")]); assert_eq!(own_pane(), "w9:p1");
    set_env(&[("HERDR_ENV", "1"), ("HERDR_PANE_ID", "w9:t2"), ("HERDR_PLUGIN_ID", "herdr-projects")]); assert_eq!(own_pane(), "w9:t2", "the plugin's own tab is a pane like any other");
    set_env(&[("HERDR_ENV", "1"), ("HERDR_PLUGIN_ID", "herdr-projects")]); assert_eq!(own_pane(), "", "a popup is no pane to go back to"); assert!(in_popup());
    set_env(&[("HERDR_PANE_ID", "w9:p1")]); assert_eq!(own_pane(), "");
    set_env(&[("HERDR_ENV", "1"), ("HERDR_PLUGIN_ID", "herdr-projects"), ("HERDR_PANE_ID", "w9:t2")]); assert!(!in_popup());
    set_env(&[("HERDR_ENV", "1"), ("HERDR_PANE_ID", "w9:p1")]); assert!(!in_popup());
    // The plugin's own entry says the same: a tab, so nothing of it is herdr's one popup.
    let toml = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("herdr-plugin.toml")).unwrap();
    assert!(has(&toml, "\\[\\[panes\\]\\]\nid = \"board\"\n[^\\[]*\nplacement = \"tab\"\n"), "{toml}");
    // No board is running: one is opened in a tab of its own, named, and herdr goes to it.
    set_env(&[]);
    let main = || sw::cli::main(vec!["board-open".into()]);
    assert_eq!(main(), 0);
    assert_eq!(w.state().s("boardPlacement"), "tab");
    let made = w.state().arr("panes").iter().find(|x| x.s("label") == "Work board").cloned().expect("a board's pane");
    assert_eq!(w.state().arr("tabs").iter().find(|t| t.s("tab_id") == made.s("tab_id")).unwrap().s("label"), "board");
    assert_eq!(focus(&w), vec!["board".to_string(), format!("workspace {}", made.s("workspace_id")), format!("tab {}", made.s("tab_id"))]);
    w.patch(|s| s["focus"] = json!(["board"]));
    // One is running in a pane: herdr goes to its space and tab, and no second board is started.
    set_env(&[("HERDR_ENV", "1"), ("HERDR_PANE_ID", "w9:p1")]); arrive(); set_env(&[]);
    assert_eq!(main(), 0); assert_eq!(focus(&w), vec!["board", "workspace w9", "tab w9:t1"]);
    // The board shares its tab with another pane that has the keyboard (the agent you went into, split beside it):
    // the keyboard goes to the board's pane, and the tab is as it was, not left zoomed.
    let zooms = |w: &World| calls(w).into_iter().filter(|c| c.starts_with("pane zoom")).collect::<Vec<_>>();
    w.patch(|s| { s["panes"].as_array_mut().unwrap().push(json!({ "pane_id": "w9:p2", "workspace_id": "w9", "tab_id": "w9:t1", "terminal_id": "term_agent", "cwd": root, "agent": null })); s["layouts"] = json!({ "w9:p1": { "zoomed": false, "focused_pane_id": "w9:p2", "panes": [] } }); });
    assert_eq!(main(), 0);
    assert_eq!(focus(&w)[focus(&w).len() - 3..].to_vec(), vec!["workspace w9", "tab w9:t1", "pane w9:p1"]);
    assert_eq!(zooms(&w), vec!["pane zoom w9:p1 --on", "pane zoom w9:p1 --off"]);
    // It has the keyboard already: nothing is zoomed, not even for a moment.
    w.patch(|s| s["layouts"]["w9:p1"]["focused_pane_id"] = json!("w9:p1"));
    assert_eq!(main(), 0); assert_eq!(focus(&w)[focus(&w).len() - 2..].to_vec(), vec!["workspace w9", "tab w9:t1"]); assert_eq!(zooms(&w).len(), 2);
    // The tab was zoomed on the other pane: the board takes the zoom, since a zoomed tab shows one pane only.
    w.patch(|s| s["layouts"]["w9:p1"] = json!({ "zoomed": true, "focused_pane_id": "w9:p2", "panes": [] }));
    assert_eq!(main(), 0); assert_eq!(zooms(&w)[2..].to_vec(), vec!["pane zoom w9:p1 --on"]);
    w.patch(|s| { s.as_object_mut().unwrap().remove("layouts"); });
    // Its pane was closed, or the board in it ended without saying so: a new one is opened.
    let notes: Vec<std::path::PathBuf> = std::fs::read_dir(sw::store::files::data_dir()).unwrap().flatten().map(|e| e.path()).filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("board-pane-")).collect();
    assert_eq!(notes.len(), 1);
    let write = |v: serde_json::Value| sw::store::files::write_json(&notes[0], &v).unwrap();
    write(json!({ "pane": "w9:p1", "pid": (1i64 << 22) + 1 })); // no process has this number
    assert!(!go_back(&w.socket));
    write(json!({ "pane": "gone:p1", "pid": std::process::id() }));
    assert!(!go_back(&w.socket));
    let n = focus(&w).len();
    assert_eq!(main(), 0); assert_eq!(focus(&w)[n], "board", "{:?}", focus(&w));
    // Another board's note is left alone; the board that wrote it takes it away when it ends.
    write(json!({ "pane": "w9:p1", "pid": 1 })); depart(); assert!(notes[0].exists());
    set_env(&[("HERDR_ENV", "1"), ("HERDR_PANE_ID", "w9:p1")]); arrive(); depart(); assert!(!notes[0].exists());
    set_env(&[]);
}
