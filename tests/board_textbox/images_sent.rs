//! A picture in what was sent stands as `[Image 1]` in the chat too (crates/board/src/board/view/talk.rs): its path,
//! whatever machine its file is on, and the `[Image #39]` Claude Code writes for one it took in, numbered per message
//! in the accent, as in the box. Your words on their way and the same words in the conversation stand once.
use crate::common;
use common::board::open;
use common::*;
use sw::agents::talk::Said;
use sw::board::textbox::images::labelled;
use sw::board::view::talk::{with_mine, Part};

const HERDR: &str = "/var/folders/gf/7lzcszfj0712mkfw2ptky5vw0000gn/T/herdr-clipboard-images-501/client-373-clipboard-1792666061783975000-0.png";
fn herdr(n: usize) -> String { HERDR.replace("-0.png", &format!("-{n}.png")) }

#[test]
fn every_picture_in_sent_words_is_its_chip_numbered_per_message() {
    let sent = format!("{} also tell hp-127 that it should never display the {}  text below", herdr(0), herdr(1));
    assert_eq!(labelled(&sent), "[Image 1] also tell hp-127 that it should never display the [Image 2]  text below");
    // What Claude Code wrote for the first, the path of the second: one count.
    assert_eq!(labelled(&format!("[Image #39] and {}", herdr(1))), "[Image 1] and [Image 2]");
    // A dropped file, escaped or in quotes, on another machine (no file here), and one under ~/.
    assert_eq!(labelled("see /Users/you/Desktop/Screen\\ Shot.PNG, '/tmp/a b.jpg' ~/.herdr-projects/inbox/x.webp"), "see [Image 1], [Image 2] [Image 3]");
    // As the web board's chat box sends one.
    assert_eq!(labelled("@/Users/you/x.png what is this"), "[Image 1] what is this");
    // No picture: as it was.
    for words in ["Look at /tmp/notes.txt and src/x.png and https://x.dev/a.png.", "[Image 2] typed by hand"] { assert_eq!(labelled(words), words); }
}

#[test]
fn the_coordinators_chat_shows_your_pictures_as_chips_in_the_accent() {
    let w = world();
    w.add("Some task");
    let mut b = open(&w.root, 30);
    b.s.side = false;
    let d = &mut b.s.data;
    d.can_dispatch = true;
    let c = &mut d.coord;
    c.state = "idle".into(); c.readable = true;
    c.talk = vec![Said { you: false, text: "Hi.".into(), at: 0 }, Said { you: true, text: format!("[Image #39] here and {}  text below", herdr(1)), at: 0 }];
    c.turn = Some(sw::agents::read_turn::Turn { id: "turn-1".into(), ..Default::default() });
    let chat = crate::board_chat::chat(&mut b, 120);
    let text = chat.boxed.join("\n");
    assert!(text.contains("[Image 1] here and [Image 2]  text below"), "{text}");
    assert!(!text.contains("herdr-clipboard-images") && !text.contains("#39"), "{text}");
    let chips = b.inked(120).into_iter().flat_map(|l| l.0).filter(|p| p.c == sw::shown::ink::Ink::Accent && p.t.starts_with("[Image")).count();
    assert_eq!(chips, 2, "each chip in the accent");
}

#[test]
fn your_words_on_their_way_and_the_same_in_the_conversation_stand_once() {
    let mine = format!("{} look", herdr(0));
    // The conversation already has them, the picture as Claude Code named it.
    let had = vec![Part { by: "You".into(), text: "[Image 1] look".into(), whole: "[Image #39] look".into(), sent: true, ..Default::default() }];
    let out = with_mine(had, &mine);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].text, "[Image 1] look");
    // Other pictures with the same words: another message, both stand.
    let had = vec![Part { by: "You".into(), text: "[Image 1] look".into(), whole: format!("{} look", herdr(1)), sent: true, ..Default::default() }];
    assert_eq!(with_mine(had, &mine).len(), 2);
    // Not yet: yours, its picture a chip.
    let out = with_mine(vec![], &mine);
    assert_eq!((out[0].text.as_str(), out[0].whole.as_str()), ("[Image 1] look", mine.as_str()));
}
