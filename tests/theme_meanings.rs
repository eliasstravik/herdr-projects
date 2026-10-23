//! Every colour the board gives a meaning of its own ("to do", an automation, a running review, pass and fail) is the
//! kind of colour it is called and told apart from everything it sits beside, in every theme light and dark, and
//! colours of your own cannot take that away.
use crate::common;
use sw::board::theme::greys::{apart, contrast, FLOOR, TODO_APART, TODO_BESIDE, TODO_DIMMER};
use sw::board::theme::picks::{is_blue, is_magenta, is_purple, AUTO_APART, AUTO_BESIDE, QUEUE_APART, QUEUE_BESIDE, REVIEW_APART, REVIEW_BESIDE, REVIEW_BLUE};
use sw::board::theme::{resolve, Pal, Seen, Theme, C};
use sw::herdr::palette::THEME_NAMES;
use sw::shown::ink::Ink;

fn cfg(s: &str) -> toml::Table { s.parse().unwrap_or_else(|e| panic!("{s}: {e}")) }
fn pal(c: &str, want: &str) -> Pal { resolve(&cfg(c), None, want, Seen::default()).pal }
fn custom(name: &str, colours: &[(&str, &str)]) -> String { format!("[theme]\nname = \"{name}\"\n[theme.custom]\n{}", colours.iter().map(|(k, v)| format!("{k} = \"{v}\"\n")).collect::<String>()) }
fn real(c: C) -> u32 { c.real().unwrap_or_else(|| panic!("{c:?} is not a real colour")) }

// One loop for every colour the board gives a meaning of its own: in every theme it is the kind of colour it is
// called, and told apart from everything it sits beside. A new such colour is one more line in the table.
type Own = (&'static str, &'static [&'static str], f64, Option<fn(C) -> bool>);
const OWN: [Own; 6] = [
    // name, what it is told apart from, by how much, what kind of colour it is (None: any)
    ("todo", &TODO_BESIDE, 45.0, None), // a task nobody started: a dim grey, never the green of ready or the blue of a review
    ("auto", &AUTO_BESIDE, AUTO_APART, Some(is_purple)), // an automation: never taken for a status, for plain or greyed words, or lost on the background
    ("review", &REVIEW_BESIDE, REVIEW_APART, Some(is_blue)), // a review that is running, and the review tag beside passed, failed and no result
    ("queue", &QUEUE_BESIDE, QUEUE_APART, Some(is_magenta)), // a task that waits (◷): never taken for red, the automations' purple or any other status
    ("green", &["red", "subtext0"], 50.0, None), // a tag's colour alone says pass, fail or no result yet
    ("red", &["green", "subtext0"], 50.0, None),
];
fn told(p: &Pal, why: &str) {
    for (name, beside, by, kind) in OWN {
        if let Some(kind) = kind { assert!(kind(p.get(name)), "{why}: {name} is the colour it is called"); }
        // The terminal's own sixteen are its own to tell apart: there, different is enough.
        for k in beside {
            let (a, b) = (p.get(name), p.get(k));
            if matches!(a, C::Ansi(_)) || matches!(b, C::Ansi(_)) { assert_ne!(a, b, "{why}: {name} and {k}"); } else { assert!(apart(a, b) >= by, "{why}: {name} and {k} are {} apart", apart(a, b).round()); }
        }
    }
}

#[test]
fn every_colour_with_a_meaning_of_its_own_is_told_apart_in_every_theme_and_colours_of_your_own_cannot_take_that_away() {
    for name in THEME_NAMES {
        let p = pal(&format!("[theme]\nname = \"{name}\"\n"), "herdr");
        told(&p, name);
        if *name == "terminal" { continue; }
        assert_ne!(p.overlay0, p.subtext0, "{name}: quiet and plain are one colour");
        // "to do" is a grey, and a dim one: plain text stands out from the background 1.3 times as much or more.
        let t = real(p.todo); let (r, g, b) = (t >> 16, (t >> 8) & 255, t & 255);
        assert!(r.max(g).max(b) - r.min(g).min(b) <= 80, "{name}: to do is a grey");
        assert!(contrast(real(p.text), real(p.panel_bg)) / contrast(t, real(p.panel_bg)) >= TODO_DIMMER, "{name}: to do is dimmer than text");
        assert!(p.review == p.blue || p.review == C::Rgb(REVIEW_BLUE), "{name}: the theme's blue, or the board's");
    }
    // The theme's own colour is taken where it fits: herdr's mauve and its blue, and a colour of your own that is one.
    assert_eq!(pal("", "herdr").auto, C::Rgb(0xcba6f7)); assert_eq!(pal("", "terminal").auto, C::Ansi(5)); assert_eq!(pal("", "terminal").review, C::Ansi(4));
    assert_eq!(pal("[theme.custom]\nmauve = \"#9333ea\"\n", "herdr").auto, C::Rgb(0x9333ea));
    assert_eq!(pal("[theme.custom]\nblue = \"#0000ff\"\n", "herdr").review, C::Rgb(0x0000ff));
    assert_eq!(pal("[theme.custom]\nblue = \"#a6e3a1\"\n", "herdr").review, C::Rgb(REVIEW_BLUE)); // one that is the theme's green is not
    // Colours of your own cannot take a meaning away: a grey that is your text colour or your green, a mauve that is the
    // theme's red, a blue that is no blue or is the red of what failed. In a dark theme, a light one and the terminal's.
    let yours: [&[(&str, &str)]; 13] = [&[("overlay0", "#cdd6f4")], &[("overlay0", "#a6e3a1"), ("overlay1", "#a6e3a1")], &[("overlay0", "#3b82f6"), ("overlay1", "#cdd6f4")], &[("mauve", "#f38ba8")], &[("mauve", "#cdd6f4")], &[("mauve", "#a6e3a1")],
        &[("blue", "#f38ba8")], &[("blue", "#f9e2af")], &[("blue", "#a6e3a1"), ("teal", "#3b82f6")], &[("blue", "#8839ef")], &[("blue", "#808080")], &[("blue", "#fe640b")], &[("blue", "#3b82f6"), ("red", "#3b82f6")]];
    for name in ["catppuccin", "catppuccin-latte"] {
        for colours in yours {
            let p = pal(&custom(name, colours), "herdr"); let why = format!("{name} {colours:?}");
            assert!(is_purple(p.auto) && is_blue(p.review) && is_magenta(p.queue), "{why}");
            for (c, beside, by) in [("todo", &TODO_BESIDE[..], TODO_APART), ("auto", &AUTO_BESIDE[..], AUTO_APART), ("review", &REVIEW_BESIDE[..], REVIEW_APART), ("queue", &QUEUE_BESIDE[..], QUEUE_APART)] {
                for k in beside { assert!(apart(p.get(c), p.get(k)) >= by, "{why}: {c} and {k}"); }
            }
            assert!(contrast(real(p.todo), real(p.panel_bg)) >= FLOOR, "{why}");
        }
    }
    { let p = pal(&custom("terminal", &[("blue", "red")]), "herdr"); assert_ne!(p.review, p.red); }
    for (c, yes) in [(C::Rgb(0x3b82f6), true), (C::Rgb(0x076678), true), (C::Ansi(4), true), (C::Rgb(0xa6e3a1), false), (C::Rgb(0x8839ef), false), (C::Rgb(0xb0b0b0), false), (C::Ansi(2), false), (C::Own, false)] { assert_eq!(is_blue(c), yes, "{c:?}"); }
}

#[test]
fn painted_each_colour_with_a_meaning_comes_out_as_a_colour_of_its_own() {
    let _w = common::world();
    for light in [false, true] {
        let t = Theme::load(Some(light), Seen::default());
        let all: Vec<C> = [Ink::Green, Ink::Red, Ink::Review, Ink::Soft, Ink::Todo, Ink::Auto, Ink::Queue, Ink::Text].iter().map(|i| t.ink(*i)).collect();
        for (i, a) in all.iter().enumerate() { for b in &all[i + 1..] { assert_ne!(a, b, "{all:?}"); } }
    }
}
