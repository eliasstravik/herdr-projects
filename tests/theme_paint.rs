//! The board paints with what it read from herdr's config: true colour, the terminal's 16 colours, herdr's marks, its
//! text colour under everything left uncoloured, and nothing painted with NO_COLOR.
use crate::common;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use sw::board::run::draw::draw;
use sw::board::theme::{Seen, Theme, C};
use sw::shown::ink::{Ink, Inked};

const STATES: [&str; 8] = ["working", "review", "board", "queue", "blocked", "done", "idle", "error"];
fn herdr_config(text: &str) { let f = sw::extras::herdr_config(); std::fs::create_dir_all(f.parent().unwrap()).unwrap(); std::fs::write(f, text).unwrap(); }
fn load(light: Option<bool>) -> Theme { Theme::load(light, Seen::default()) }
/// The line drawn: each cell's text colour, background and attributes.
fn cells(t: &Theme, line: Inked) -> Vec<(Color, Color, Modifier)> { let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1)); draw(t, &[line], &mut buf); (0..8).map(|x| { let c = &buf[(x, 0)]; (c.fg, c.bg, c.modifier) }).collect() }
fn fg(t: &Theme, i: Ink) -> Color { cells(t, Inked::of("x", i))[0].0 }
fn bg(t: &Theme, i: Ink) -> Color { cells(t, Inked::plain("x").on(i))[0].1 }
fn marks(t: &Theme) -> Vec<&'static str> { STATES.iter().map(|s| t.mark(s, "")).collect() }

#[test]
fn the_board_paints_with_what_it_read_true_colour_the_terminals_16_colours_herdrs_marks() {
    let _w = common::world();
    unsafe { std::env::remove_var("HERDR_CONFIG_PATH") };
    herdr_config("[theme]\nname = \"terminal\"\nauto_switch = true\nlight_name = \"catppuccin-latte\"\n[ui]\nstatus_indicators = \"symbols\"\n");
    assert!(Theme::needs_light());
    let t = load(None);
    assert_eq!(fg(&t, Ink::Red), Color::Indexed(9)); assert_eq!(fg(&t, Ink::Green), Color::Indexed(2)); assert_eq!(fg(&t, Ink::Text), Color::Reset);
    assert_eq!(fg(&t, Ink::Yellow), Color::Indexed(3)); assert_eq!(fg(&t, Ink::Plain), Color::Reset);
    assert_eq!(fg(&t, Ink::Muted), Color::Reset); // this theme's greys cannot be trusted, nor can faint: untold, greyed-out words are the text colour
    assert_eq!(fg(&t, Ink::Soft), Color::Reset); // and words that have to be read the text colour itself
    assert!(t.paints_selection()); assert_eq!(bg(&t, Ink::Selection), Color::Indexed(8)); // no selection colour in this theme: herdr's active row colour
    assert_eq!(marks(&t), ["◐", "◑", "◇", "◷", "!", "✓", "○", "✗"]);
    let t = load(Some(true));
    assert_eq!(fg(&t, Ink::Red), Color::Rgb(210, 15, 57)); assert_eq!(bg(&t, Ink::Selection), Color::Rgb(189, 208, 245));
    herdr_config("");
    assert!(!Theme::needs_light()); // no theme set: herdr's default, with nothing to ask
    let t = load(None);
    // A symbol of its own for each state is the board's default: no two states are told apart by colour alone.
    assert_eq!(marks(&t), ["◐", "◑", "◇", "◷", "!", "✓", "○", "✗"]);
    // A task that waits for you says what it needs: an answer, a yes or no, a sign-in; `!` where the kind is not known.
    // None of them is a cross: that is for what went wrong, and told from it by the mark alone, with colour off too.
    let needs: Vec<&str> = ["question", "permission", "auth", "", "nonsense"].iter().map(|k| t.mark("blocked", k)).collect();
    assert_eq!(needs, ["?", "!", "@", "!", "!"]); assert!(needs.iter().all(|m| !"×✗✗".contains(m) && *m != t.mark("error", "")));
    assert_eq!(t.mark("error", "question"), "✗"); assert_eq!(t.mark("working", "auth"), "◐"); // the kind belongs to blocked only
    herdr_config("[ui]\nstatus_indicators = \"dots\"\n"); // herdr's dots only when its config asks for them by name
    let t = load(None);
    assert_eq!(marks(&t), ["●", "●", "◇", "◷", "!", "●", "○", "✗"]); // herdr's dot for someone at work, as a moon is with symbols
    assert_eq!(["question", "permission", "auth", ""].map(|k| t.mark("blocked", k)), ["?", "!", "@", "!"]); // what a task needs is said with dots too
    herdr_config("");
    assert_eq!(fg(&load(None), Ink::Yellow), Color::Rgb(249, 226, 175)); // no theme set here: herdr's default, as herdr shows it
    herdr_config("[theme]\nname = \"terminal\"\n");
    // The terminal theme: the terminal's colours and its own text colour; a task nobody started is that text colour too, until the terminal tells its colours.
    assert_eq!(fg(&load(None), Ink::Todo), Color::Reset);
    let t = Theme::load(None, Seen { light: None, fg: Some(0xd0d7e5), bg: Some(0x0b1a33) }); // the terminal told its colours: a grey mixed from the two, measured
    assert_eq!(fg(&t, Ink::Todo), Color::Rgb(122, 133, 151)); assert_eq!(fg(&t, Ink::Muted), fg(&t, Ink::Todo)); assert_eq!(fg(&t, Ink::Text), Color::Reset);
    let t = load(None);
    assert_eq!(fg(&t, Ink::Yellow), Color::Indexed(3)); assert_eq!(fg(&t, Ink::Teal), Color::Indexed(6)); assert!(cells(&t, Inked::plain("a b")).iter().all(|c| c.0 == Color::Reset));
    herdr_config("[theme]\nname = \"catppuccin\"\n");
    assert!(!Theme::needs_light()); // no auto_switch: nothing to ask
    let t = load(None);
    assert_eq!(fg(&t, Ink::Muted), Color::Rgb(126, 131, 155)); // a theme with a grey of its own: that grey, made readable
    assert_eq!(t.pal.get("nonesuch"), C::Own); assert!(cells(&t, Inked::plain("x").dim())[0].2.contains(Modifier::DIM));
    assert_eq!(fg(&t, Ink::Yellow), Color::Rgb(249, 226, 175));
    // Uncoloured text is herdr's text colour, and stays so after a coloured word, to the line's end.
    let text = Color::Rgb(205, 214, 244);
    let mut l = Inked::plain("a "); l.push("b", Ink::Red).push(" c", Ink::Plain);
    assert_eq!(cells(&t, l).iter().map(|c| c.0).collect::<Vec<_>>(), [text, text, Color::Rgb(243, 139, 168), text, text, text, text, text]);
    assert_eq!(bg(&t, Ink::Sidebar), Color::Reset); // Catppuccin leaves the sidebar on the terminal's background
    herdr_config("[theme]\ncustom = { text = \"#010203\", sidebar_bg = \"#040506\" }\n");
    let t = load(None);
    assert_eq!(fg(&t, Ink::Plain), Color::Rgb(1, 2, 3));
    // A background stays on beside another one inside it.
    let mut l = Inked::plain("a").on(Ink::Sidebar); l = l.add(Inked::plain("b").on(Ink::Selection)).add(Inked::plain("c").on(Ink::Sidebar));
    let bgs: Vec<Color> = cells(&t, l).iter().take(3).map(|c| c.1).collect();
    assert_eq!(bgs, [Color::Rgb(4, 5, 6), t.ink(Ink::Selection).color(), Color::Rgb(4, 5, 6)]); assert_ne!(bgs[1], Color::Reset);
    unsafe { std::env::set_var("NO_COLOR", "1") };
    let t = load(None);
    let mut l = Inked::plain("a "); l.push("b", Ink::Red);
    assert!(cells(&t, l).iter().all(|c| c.0 == Color::Reset && c.1 == Color::Reset));
    assert_eq!(bg(&t, Ink::Sidebar), Color::Reset);
    // Where the old board left the selected row and bold words plain with colour off, this board draws the selected row
    // reversed and keeps bold: neither is a colour, and both still say what colour said (theme/mod.rs, draw.rs).
    assert!(!t.paints_selection()); assert_eq!(fg(&t, Ink::Accent), Color::Reset);
    unsafe { std::env::remove_var("NO_COLOR") };
}

#[test]
fn only_a_moon_says_someone_works_and_every_mark_is_one_column_wide() {
    use sw::board::theme::marks::{mark, web_mark, MARK_STATES};
    let kinds = ["", "question", "permission", "auth"];
    let all: Vec<(&str, &str, &str)> = MARK_STATES.iter().flat_map(|s| kinds.iter().flat_map(move |k| [("symbols", *s, mark(s, k, true)), ("dots", *s, mark(s, k, false)), ("web", *s, web_mark(s, k))])).collect();
    for (how, state, m) in &all {
        assert_eq!((sw::text::width(m), m.chars().count()), (1, 1), "{how} {state}: {m}");
        let mut buf = Buffer::empty(Rect::new(0, 0, 3, 1)); draw(&Theme::default_dark(true), &[Inked::plain(*m).add(Inked::plain("x"))], &mut buf);
        assert_eq!(buf[(1, 0)].symbol(), "x", "{how} {state}: {m} takes one cell on the terminal board");
        let moon = "◐◑◒◓".contains(*m);
        assert_eq!(moon, ["working", "review"].contains(state) && *how != "dots" && !(*how == "web" && *state == "working"), "{how} {state}: {m}");
    }
    assert_eq!(["board", "queue"].map(|s| mark(s, "", true)), ["◇", "◷"]); assert_eq!(["board", "queue"].map(|s| web_mark(s, "")), ["◇", "◷"]);
}
