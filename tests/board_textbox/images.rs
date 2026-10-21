//! A picture pasted into the text box (crates/board/src/board/textbox/images.rs): herdr's clipboard path, or a picture
//! file dropped, stands as `[Image 1]`, `[Image 2]` (numbered per message, from 1 again after it is sent), moves and
//! deletes as one character, comes back with undo, and is its path again in what is sent and kept as the draft. Words
//! that are not only pictures' paths go in as they are.
use crate::common;
use common::board::open;
use common::*;

const HERDR: &str = "/var/folders/gf/7lzcszfj0712mkfw2ptky5vw0000gn/T/herdr-clipboard-images-501/client-327-clipboard-1792608324073248000-0.png";
fn herdr(n: usize) -> String { HERDR.replace("-0.png", &format!("-{n}.png")) }
/// The words as the box holds them, checked against its rows on the screen.
fn shown(s: &mut common::board::Board) -> String { let rows = s.draft(120).rows.join("\n"); let raw = s.s.input.raw(); assert_eq!(rows, raw.trim_end(), "on the screen as in the box"); raw }

#[test]
fn herdrs_clipboard_path_stands_as_image_1_and_is_sent_as_the_path() {
    let w = world(); w.add("Some task");
    let mut s = open(&w.root, 30);
    s.typed("look at");
    s.paste(&herdr(0));
    assert_eq!(shown(&mut s), "look at [Image 1] ", "a space on either side of the chip");
    s.typed("and");
    s.paste(&format!("{}\n", herdr(1)));
    assert_eq!(shown(&mut s), "look at [Image 1] and [Image 2] ");
    assert_eq!(s.s.value(), format!("look at {} and {} ", herdr(0), herdr(1)), "the words name each path again");
    assert!(!s.text().contains("herdr-clipboard-images"), "the path is never on the screen");
    let chips = s.inked(120).into_iter().flat_map(|l| l.0).filter(|p| p.c == sw::shown::ink::Ink::Accent && p.t.starts_with("[Image")).count();
    assert_eq!(chips, 2, "each chip in the accent");
    // Sent (here: added as a task): the next message counts from 1 again.
    s.s.input.set_text("");
    s.paste(&herdr(2));
    assert_eq!(shown(&mut s), "[Image 1] ");
    assert_eq!(s.s.value(), format!("{} ", herdr(2)));
}

#[test]
fn a_chip_is_one_character_to_the_arrows_and_backspace_and_undo_brings_it_back() {
    let w = world(); w.add("Some task");
    let mut s = open(&w.root, 30);
    s.typed("a ");
    s.paste(&herdr(0));
    s.typed("b");
    assert_eq!(shown(&mut s), "a [Image 1] b");
    s.press(&["left", "left", "left"]); // past b, the space, and the whole chip
    assert_eq!(s.s.input.cursor().1, 2);
    s.press(&["right"]);
    assert_eq!(s.s.input.cursor().1, "a [Image 1]".len());
    s.press(&["backspace"]);
    assert_eq!(shown(&mut s), "a  b", "backspace takes the whole chip");
    assert_eq!(s.s.value(), "a  b");
    s.press(&["ctrl+-"]);
    assert_eq!(shown(&mut s), "a [Image 1] b");
    assert_eq!(s.s.value(), format!("a {} b", herdr(0)));
    s.press(&["home", "right", "right", "delete"]);
    assert_eq!(shown(&mut s), "a  b", "delete too");
    s.press(&["ctrl+-", "end", "left", "left", "ctrl+w"]);
    assert_eq!(shown(&mut s), "a  b", "and a word back");
}

#[test]
fn a_dropped_picture_file_is_a_chip_too_and_other_pastes_stay_words() {
    let w = world(); w.add("Some task");
    let dir = w.base.join("desk");
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["Screen Shot.png", "b.JPG", "notes.txt"] { std::fs::write(dir.join(f), b"x").unwrap(); }
    let shot = dir.join("Screen Shot.png").display().to_string().replace(' ', "\\ ");
    let b = dir.join("b.JPG").display().to_string();
    let mut s = open(&w.root, 30);
    // Two dropped at once, as a terminal gives them: escaped, space between.
    s.paste(&format!("{shot} '{b}'"));
    assert_eq!(shown(&mut s), "[Image 1] [Image 2] ");
    assert_eq!(s.s.value(), format!("{shot} '{b}' "), "exactly as pasted");
    // Not only pictures, a file that is not one, or a picture that is not there: words, as pasted.
    for words in [format!("see {b}"), dir.join("notes.txt").display().to_string(), dir.join("gone.png").display().to_string()] {
        s.s.input.set_text("");
        s.paste(&words);
        assert_eq!(s.s.value(), words);
        assert!(!shown(&mut s).contains("[Image"), "{words}");
    }
    // Words typed that look like a chip, for a picture this message has not: words.
    s.s.input.set_text("");
    s.typed("[Image 1]");
    assert_eq!(s.s.value(), "[Image 1]");
}

#[test]
fn a_draft_that_comes_back_has_its_chips_again() {
    let w = world(); w.add("Some task");
    let mut s = open(&w.root, 30);
    s.typed("look");
    s.paste(&herdr(0));
    sw::board::run::prefs::save(&s.s);
    let mut s = open(&w.root, 30);
    assert_eq!(shown(&mut s), "look [Image 1] ");
    assert_eq!(s.s.value(), format!("look {} ", herdr(0)));
}
