//! Your own keys (`board.keys`): a moved key answers to your chord only, and the hint line, the ? menu and the board's
//! messages name it so; a chord another key has, or the key that opens the board, is refused with the key it clashes
//! with; it is yours only; docs/keys.md keeps the built-in chords.
use crate::common;
use common::board::open;
use common::*;
use serde_json::json;
use sw::actions::settings::{settings_command, Where};
use sw::board::keys::{is, key_name, Press, K};
use sw::board::remap::read;

fn you() -> Where { Where { task: None, repo: None, automation: None, coord: None } }
fn set(v: &str) -> Result<String, String> {
    settings_command("set", &["set".into(), "board.keys".into()], &you(), Some(json!(v))).map(|r| r.1).map_err(|e| e.msg)
}
fn v(a: &[&str]) -> Vec<String> { a.iter().map(|s| s.to_string()).collect() }

#[test]
fn a_moved_key_answers_to_your_chord_only_and_the_board_names_it_so() {
    let w = world_with(Opts { repo: true, settings: json!({ "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "f.txt": "1\n" }, "commit": "1", "note": "Made f." }] }));
    let id = w.add("Make f");
    delegate(&id); settle(&id);
    assert_eq!(key_name("chatMessage"), "alt+c");
    assert_eq!(set("chatMessage = Ctrl+G, chatScrollUp=ctrl+up chatScrollDown=ctrl+down").unwrap().lines().next().unwrap(), "board.keys: chatMessage=ctrl+g, chatScrollUp=ctrl+up, chatScrollDown=ctrl+down  (you)");
    assert_eq!((key_name("chatMessage"), key_name("chatScrollUp")), ("ctrl+g".to_string(), "ctrl+up".to_string()));
    assert!(is(&Press::chord("ctrl+g"), K::ChatMessage) && !is(&Press::chord("alt+c"), K::ChatMessage));
    let mut s = open(&w.root, 34); s.s.side = false;
    s.s.sel = id.clone(); s.load(); s.press(&["space"]);
    assert!(s.chat(120).hints.ends_with("ctrl+g hide the latest message · space back to the Coordinator"), "{}", s.chat(120).hints);
    s.press(&["alt+c"]); assert!(s.chat(120).framed, "the old chord does nothing now");
    s.press(&["ctrl+g"]);
    let c = s.chat(120);
    assert!(!c.framed); assert_eq!(c.boxed, v(&["❯ Send message to Agent"]));
    assert!(c.hints.ends_with("ctrl+g show the latest message · space back to the Coordinator"), "{}", c.hints);
    s.press(&["ctrl+g"]); assert!(s.chat(120).framed);
    s.press(&["?"]);
    let t = s.text();
    assert!(regex::Regex::new(r"ctrl\+g +hide the chat's latest message").unwrap().is_match(&t), "{t}");
    assert!(!t.contains("alt+c"), "{t}");
    // Taken away, the key is its own again.
    settings_command("unset", &["unset".into(), "board.keys".into()], &you(), None).unwrap();
    assert_eq!(key_name("chatMessage"), "alt+c");
    assert!(is(&Press::chord("alt+c"), K::ChatMessage));
}

#[test]
fn a_chord_that_clashes_is_refused_naming_the_key_it_clashes_with() {
    let _w = world();
    let e = set("chatMessage=ctrl+l").unwrap_err();
    assert!(e.contains("chatMessage=ctrl+l: ctrl+l is yes's key already. Move yes too, or choose another") && e.ends_with("Nothing was changed."), "{e}");
    assert_eq!(key_name("chatMessage"), "alt+c");
    assert!(set("stop=ctrl+q, archive=ctrl+q").unwrap_err().contains("archive=ctrl+q is the same chord"));
    // Moving the other one too frees its chord, in either order.
    set("undo=ctrl+c, close=ctrl+q").unwrap();
    assert_eq!((key_name("undo"), key_name("close")), ("ctrl+c".to_string(), "ctrl+q".to_string()));
    assert!(is(&Press::chord("ctrl+c"), K::Undo) && is(&Press::chord("ctrl+q"), K::Close) && !is(&Press::chord("ctrl+z"), K::Undo));
    let e = set("chatMesage=ctrl+o").unwrap_err(); assert!(e.contains("no key is called chatMesage. Did you mean chatMessage?"), "{e}");
    let e = set("stop=x").unwrap_err(); assert!(e.contains("a plain x is typed into the text box"), "{e}");
    let e = set("newline=ctrl+j").unwrap_err(); assert!(e.contains("newline is the text box's own key"), "{e}");
    let e = set("chatMessage").unwrap_err(); assert!(e.contains("\"chatMessage\" is not a key's name and a chord"), "{e}");
    let e = set("chatMessage=ctrl+").unwrap_err(); assert!(e.contains("is not a chord"), "{e}");
    let e = set("talk=t, sayYes=t").unwrap_err(); assert!(e.contains("sayYes=t: talk=t is the same chord") || e.contains("talk=t: sayYes=t is the same chord"), "{e}");
    set("talk=t").unwrap(); assert!(is(&Press::char('t'), K::Talk) && is(&Press::char('T'), K::Talk) && !is(&Press::char(' '), K::Talk));
    // A function key in the one spelling a terminal sends it in.
    assert!(set("menu=F01").unwrap().starts_with("board.keys: menu=f1  (you)"));
    assert!(is(&Press::chord("f1"), K::Menu) && key_name("menu") == "f1");
    // The key that opens the board: itself, or herdr's prefix when it starts with that.
    let (_, bad) = read(&json!("chatMessage=alt+w"), &Some((vec![], "alt+w".into())));
    assert_eq!(bad, v(&["chatMessage=alt+w: alt+w opens the board"]));
    let (moved, bad) = read(&json!("chatMessage=ctrl+b, side=alt+o"), &Some((vec!["ctrl+b".into()], "a".into())));
    assert_eq!(bad, v(&["chatMessage=ctrl+b: ctrl+b is herdr's prefix, which the key that opens the board starts with (prefix+a)"]));
    assert_eq!(moved.keys().copied().collect::<Vec<_>>(), vec!["side"]);
}

#[test]
fn it_is_yours_only_a_hand_written_mistake_is_left_out_and_the_page_keeps_the_built_in_chords() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.keys": "chatMessage=ctrl+l, side=alt+o" }), ..Default::default() });
    let page = sw::board::remap::as_built(|| sw::board::keys::key_name("side"));
    assert_eq!(page, "alt+p");
    assert_eq!((key_name("side"), key_name("chatMessage")), ("alt+o".to_string(), "alt+c".to_string()), "the pair that clashes is left out");
    let (j, shown) = settings_command("show", &["show".into()], &you(), None).unwrap();
    assert!(shown.contains("Your keys not counted (the key keeps its own chord):\n  board.keys: chatMessage=ctrl+l: ctrl+l is yes's key already"), "{shown}");
    assert!(j.to_string().contains("ctrl+l is yes's key already"));
    let e = settings_command("set", &["set".into(), "board.keys".into()], &Where { task: None, repo: Some("proj".into()), automation: None, coord: None }, Some(json!("side=alt+o"))).unwrap_err().msg;
    assert!(e.contains("board.keys is a board setting: it is yours only"), "{e}");
    std::fs::write(w.root.join(".herdr-projects.json"), r#"{ "board.keys": "side=alt+o" }"#).unwrap();
    let (_, said) = sw::store::settings::places::repo_file(&w.root, None);
    assert!(said.iter().any(|p| p.contains("board.keys is refused")), "{said:?}");
}

#[test]
fn a_hand_written_chord_of_the_key_that_opens_the_board_does_not_count_on_the_board_either() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.keys": "menu=alt+w, side=alt+o" }), ..Default::default() });
    let conf = sw::extras::herdr_config();
    std::fs::create_dir_all(conf.parent().unwrap()).unwrap();
    let bind = |key: &str| std::fs::write(&conf, format!("[[keys.command]]\nkey = \"{key}\"\ntype = \"plugin_action\"\ncommand = \"herdr-projects.open\"\n")).unwrap();
    bind("alt+w");
    assert_eq!((key_name("menu"), key_name("side")), ("?".to_string(), "alt+o".to_string()));
    assert!(!is(&Press::chord("alt+w"), K::Menu) && is(&Press::char('?'), K::Menu));
    let mut s = open(&w.root, 34);
    s.press(&["alt+w"]); assert!(s.s.menu.is_none(), "alt+w is the board's own key: the menu stays closed");
    s.press(&["?"]); assert!(s.s.menu.is_some() && !s.text().contains("alt+w"), "{}", s.text());
    let (_, shown) = settings_command("show", &["show".into()], &you(), None).unwrap();
    assert!(shown.contains("board.keys: menu=alt+w: alt+w opens the board"), "{shown}");
    // Once the board opens on another key, the chord is yours.
    bind("prefix+a");
    assert_eq!(key_name("menu"), "alt+w");
    assert!(is(&Press::chord("alt+w"), K::Menu));
}
