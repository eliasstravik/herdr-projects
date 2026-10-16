//! The first run: one screen with both choices (the board's key, herdr's sidebar), answered with single keys; a key
//! herdr or your config uses refused as it is typed; esc asks again next time. And a board that loads its new version
//! by itself once nothing is half typed. What the config already holds, in any TOML spelling: sidebar rows of your
//! own are kept and never offered over, and the key that opens the board is found.
use crate::common;
use common::board::open;
use common::*;
use sw::board::state::Setup;
use sw::json::*;
use sw::board::update::{update, Effect, Event};

fn flat(t: &str) -> String { regex::Regex::new(r"\s+").unwrap().replace_all(t, " ").into_owned() }
fn start(w: &World, key: Option<(&str, &str)>, sidebar: bool) -> common::board::Board {
    let mut b = open(&w.root, 30);
    b.cols = 100;
    let file = sw::extras::herdr_config().display().to_string();
    b.s.setup = Some(Setup { file, typing: false, key: key.map(|(k, n)| (true, k.to_string(), n.to_string())), sidebar: sidebar.then_some(true) });
    b
}

#[test]
fn the_first_run_is_one_screen_with_both_choices_answered_with_single_keys() {
    let w = world();
    let mut s = start(&w, Some(("prefix+a", "")), true);
    let t = s.text();
    assert!(t.contains("herdr-projects · first run") && !t.contains("question 1 of"), "{t}");
    assert!(t.contains("1  [x] Open this board with prefix+a, from anywhere in herdr") && t.contains("2  [x] Show each task's stage"), "{t}");
    assert!(t.contains("1 2 on or off · enter save · esc ask me next time"), "{t}");
    s.press(&["2"]); assert!(s.text().contains("2  [ ] Show each task's stage"));
    // a key herdr uses is refused as it is typed, with what it does today
    s.press(&["k"]); s.typed("prefix+w"); s.press(&["enter"]);
    assert!(flat(&s.text()).contains("herdr itself uses prefix+w"), "{}", s.text());
    assert!(s.s.setup.as_ref().unwrap().typing && s.text().contains("Open this board with prefix+a"));
    s.typed("prefix+y"); s.press(&["enter"]);
    assert!(s.text().contains("Open this board with prefix+y"), "{}", s.text());
    s.press(&["2", "enter"]);
    assert!(s.s.setup.is_none(), "{}", s.text());
    let config = read(&sw::extras::herdr_config());
    assert!(config.contains("prefix+y"), "{config}");
    assert!(flat(&s.text()).contains("prefix+y opens the board; herdr's sidebar shows each task's stage"), "{}", s.text());
    // esc changes nothing and asks again next time
    let mut s = start(&w, Some(("prefix+b", "")), true);
    s.press(&["esc"]);
    assert!(s.s.setup.is_none() && s.text().contains("Asked again next time. Nothing was changed."));
}

#[test]
fn after_an_update_the_board_loads_its_new_version_by_itself_once_nothing_is_half_typed() {
    let w = world();
    let mut s = open(&w.root, 34);
    s.s.ctx.can_reload = true;
    s.typed("a draft");
    let fx = update(&mut s.s, Event::Stamp("new".into())); assert!(!fx.contains(&Effect::Reload));
    let fx = update(&mut s.s, Event::Stamp("new".into())); assert!(!fx.contains(&Effect::Reload), "never under a draft");
    assert!(s.text().contains("herdr-projects was updated: the new version loads when nothing is half typed."));
    s.press(&["esc"]);
    let fx = update(&mut s.s, Event::Stamp("new".into()));
    assert!(fx.contains(&Effect::Reload));
}

#[test]
fn sidebar_rows_of_your_own_in_any_spelling_are_kept_and_ours_are_given_to_paste() {
    let w = world();
    let file = sw::extras::herdr_config();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let rows = "rows = [[\"state_icon\", \"workspace\"]]";
    let own = [
        ("an inline table under [ui]", format!("[ui]\nsidebar = {{ agents = {{ {rows} }} }}\n")),
        ("a dotted key under [ui]", format!("[ui]\nmouse_capture = true\nsidebar.agents.{rows}\n")),
        ("a top-level dotted key", format!("ui.sidebar.spaces.{rows}\n")),
        ("a table header", format!("[ui.sidebar.agents]\n{rows}\n")),
    ];
    for (how, text) in &own {
        assert!(sw::extras::found::has_sidebar(text), "{how}");
        std::fs::write(&file, text).unwrap();
        let o = sw::extras::offers();
        assert!(o.at("sidebar").is_null(), "{how}: never offered");
        let note = o.t("sidebarKept");
        assert!(note.starts_with("Your own sidebar rows in ") && note.lines().next().unwrap().contains("are kept"), "{how}: {note}");
        assert!(note.contains("[ui.sidebar.agents]") && note.contains("[ui.sidebar.spaces]") && !note.contains("# >>>"), "{how}: ours to paste: {note}");
    }
    // Nothing of its own, or only the plugin's own block: offered as always, nothing said.
    for text in ["[ui]\nmouse_capture = true\n".to_string(), "[ui\nbroken".to_string(), format!("{}\n", sw::extras::block_text("sidebar", ""))] {
        std::fs::write(&file, &text).unwrap();
        let o = sw::extras::offers();
        assert!(o.at("sidebarKept").is_null(), "{text}");
        assert_eq!(o.at("sidebar").is_null(), text.contains("# >>>"), "{text}");
    }
    // A file that does not parse is searched for the usual spelling.
    assert!(sw::extras::found::has_sidebar("[ui.sidebar.agents]\nrows = [[\n") && !sw::extras::found::has_sidebar("[ui]\nsidebar = {\n"));
    // A no to the sidebar is not asked about again, and not told either.
    std::fs::write(&file, &own[0].1).unwrap();
    sw::extras::answer(serde_json::json!({ "sidebarAnswer": "no" }));
    assert!(sw::extras::offers().at("sidebarKept").is_null());
    drop(w);
}

#[test]
fn the_key_that_opens_the_board_is_found_in_any_spelling() {
    let w = world();
    let file = sw::extras::herdr_config();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let open = "command = \"herdr-projects.open\"";
    for (how, text) in [
        ("an array table", format!("[[keys.command]]\nkey = \"prefix+w\"\ntype = \"plugin_action\"\n{open}\n")),
        ("an inline array under [keys]", format!("[keys]\ncommand = [{{ key = \"prefix+w\", type = \"plugin_action\", {open} }}]\n")),
        ("a top-level dotted key", format!("keys.command = [{{ key = \"alt+x\", {open} }}, {{ key = \"prefix+w\", type = \"plugin_action\", {open} }}]\n")),
        ("after another command", format!("[[keys.command]]\nkey = \"alt+x\"\ncommand = \"other.thing\"\n\n[[keys.command]]\nkey = \"prefix+w\"\n{open}\n")),
    ] {
        std::fs::write(&file, &text).unwrap();
        let want = if how == "a top-level dotted key" { "alt+x" } else { "prefix+w" };
        assert_eq!(sw::extras::board_key(), want, "{how}");
        assert!(sw::extras::offers().at("key").is_null(), "{how}: bound already, never offered");
    }
    std::fs::write(&file, "[keys]\nx = \"prefix+x\"\n").unwrap();
    assert_eq!(sw::extras::board_key(), "");
    // A file that does not parse: the usual spelling still is.
    assert_eq!(sw::extras::found::bound_key(&format!("[[keys.command]]\nkey = \"prefix+w\"\n{open}\n[broken\n")), "prefix+w");
    drop(w);
}

#[test]
fn text_to_paste_is_whole_in_a_small_window_the_arrows_scroll_it_and_any_other_key_puts_it_away() {
    let w = world();
    let file = sw::extras::herdr_config();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "[ui]\nsidebar = { agents = { rows = [[\"workspace\"]] } }\n").unwrap();
    let note = sw::extras::offers().t("sidebarKept");
    let mut b = open(&w.root, 15);
    b.cols = 60;
    b.s.paste_text = note.clone();
    // Every line of it is on some screen, down to its last, and the keys that show the rest are always named.
    let mut seen = String::new();
    for _ in 0..40 {
        let t = b.text();
        assert!(flat(&t).contains("↑ ↓ pgup pgdn more · lines") && flat(&t).contains("any other key continues"), "{t}");
        seen.push_str(&t);
        if b.s.paste_at == b.s.paste_more { break; }
        b.press(&["down"]);
    }
    assert!(b.s.paste_more > 0 && b.s.paste_at == b.s.paste_more, "scrolled to the end");
    let end = flat(&b.text());
    assert!(end.contains(&format!("of {} ·", b.s.paste_more + b.s.paste_room)) && end.contains("\"git_status\"], [{ token = \"$summary\", dim = true }, \"$progress\"]]"), "the last line: {end}");
    assert!(seen.contains("[ui.sidebar.spaces]") && flat(&seen).contains("\"$progress\"]]"), "its last line: {seen}");
    b.press(&["pageUp", "pageUp", "pageUp", "pageUp", "pageUp"]);
    assert_eq!(b.s.paste_at, 0, "back to the top");
    assert!(b.text().contains("Your own sidebar rows"));
    b.press(&["x"]);
    assert!(b.s.paste_text.is_empty() && !b.text().contains("Your own sidebar rows"), "any other key puts it away");
    // Text that fits says only that any key continues, and the arrows put it away too.
    let mut b = open(&w.root, 40);
    b.cols = 100;
    b.s.paste_text = "short".into();
    assert!(b.text().contains("any key continues"));
    b.press(&["down"]);
    assert!(b.s.paste_text.is_empty());
}
