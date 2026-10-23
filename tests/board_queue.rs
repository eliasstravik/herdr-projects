//! Tasks that wait for a free slot: their own mark (◷, a clock) in a magenta no other state has, and their place in the queue (Q#1 gets the next
//! slot) the same on the row, the task panel, `list` and `details`, herdr's sidebar and the web Board; last under
//! `working`, in the order they start; and the places move up when a slot frees.
use crate::common;
use common::board::{open, web};
use common::*;
use serde_json::json;
use sw::json::*;
use sw::shown::ink::Ink;

fn line_of<'a>(lines: &'a [String], title: &str) -> (usize, &'a String) { lines.iter().enumerate().find(|(_, l)| l.contains(title)).unwrap_or_else(|| panic!("no row {title}:\n{}", lines.join("\n"))) }

#[test]
fn tasks_that_wait_for_a_free_slot_have_their_own_mark_and_place_in_the_queue_everywhere() {
    let w = world_with(Opts { repo: true, settings: json!({ "board.agents": 1, "review.when": "never" }), ..Default::default() });
    w.script(json!({ "worker": [{ "files": { "1.txt": "1\n" }, "commit": "1", "note": "One." }, { "status": "working" }] }));
    let (a, b, c) = (w.add("First at work"), w.add("Second waits"), w.add("Third waits"));
    delegate(&a); sw::clock::advance(1000); delegate(&b); sw::clock::advance(1000); delegate(&c);
    assert_eq!((task(&b).t("stage"), task(&c).t("stage")), ("queued".into(), "queued".into()));
    // The place is the order they get a slot: the first to wait first.
    assert_eq!((row(&b).i("queuePlace"), row(&c).i("queuePlace")), (1, 2));
    assert!(!row(&a).has("queuePlace"));
    // Their own mark, a clock, in a magenta of their own, and their place where a task at work has its percent.
    let r = sw::shown::states::as_row(&row(&c));
    assert_eq!(sw::shown::states::status_ink(&r), Ink::Queue);
    assert_eq!(sw::shown::row_says::row_fields(&r, sw::clock::now()).percent.text(), "Q#2");
    assert_eq!(sw::shown::states::standing(&r), "waiting for a free slot, Q#2");
    let mut bd = open(&w.root, 40);
    let lines = bd.screen(120);
    let (at_b, lb) = line_of(&lines, "Second waits");
    let (at_c, lc) = line_of(&lines, "Third waits");
    let (at_a, _) = line_of(&lines, "First at work");
    assert!(lb.contains('◷') && lb.trim_end().contains("Q#1") && lc.contains('◷') && lc.contains("Q#2"), "{lb}\n{lc}");
    assert!(at_a < at_b && at_b < at_c, "the waiting ones last under working, Q#1 first:\n{}", lines.join("\n"));
    assert!(lines[at_b + 1].contains("Waiting for a free slot"), "{}", lines[at_b + 1]);
    // A narrow board says the place with the time, at the end of the second line: the row stays two lines.
    assert!(bd.screen(44).iter().any(|l| l.contains("Waiting for a free slot") && l.contains("Q#2")), "{}", bd.screen(44).join("\n"));
    // The task panel's State line.
    let panel = sw::board::view::panel::panel(&mut bd.s, &bd.theme, &r, 100, false, 0).iter().map(|l| l.text()).collect::<Vec<_>>().join("\n");
    assert!(panel.contains("◷ waiting for a free slot, Q#2"), "{panel}");
    // `list` and `details`.
    let (_, said) = sw::cli::tasks::command("list", &sw::cli::flags::parse(&["list".to_string()])).unwrap();
    assert!(said.lines().any(|l| l.contains("Third waits") && l.contains("waiting for a free slot, Q#2")), "{said}");
    let d = sw::actions::finish_line::details(&c).unwrap();
    assert_eq!((d.at("queue").i("place"), d.at("queue").s("words")), (2, "waiting for a free slot, Q#2"));
    // herdr's sidebar: where it stands, then its place, without the row's second line.
    let e = sw::flow::sidebar_entry::entry_of(&row(&c), "", sw::clock::now());
    assert_eq!((e.place.as_str(), e.line.as_str(), e.progress.as_str()), ("queued", "", "Q#2"));
    // The web Board: the same mark, colour and place.
    let s = web(&w, "board", json!({}));
    let rows: Vec<Value> = s.at("list").arr("groups").iter().flat_map(|g| g.arr("rows").iter().cloned()).collect();
    let wc = rows.iter().find(|x| x.s("id") == c).unwrap();
    assert_eq!((wc.s("mark"), wc.s("color"), wc.s("percent")), ("◷", "queue", "Q#2"));
    // A slot frees: the first in the queue starts, and the one after it moves up.
    settle(&a);
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    assert_ne!(task(&b).s("stage"), "queued");
    assert_eq!(row(&c).i("queuePlace"), 1);
    assert!(!row(&b).has("queuePlace"));
}

#[test]
fn the_queue_colour_is_a_magenta_of_its_own_in_every_theme() {
    use sw::board::theme::picks::{is_magenta, QUEUE_APART, QUEUE_BESIDE};
    use sw::board::theme::{greys::{apart, contrast, LEAST}, resolve, Seen};
    for name in sw::herdr::palette::THEME_NAMES {
        let cfg: toml::Table = format!("[theme]\nname = \"{name}\"\n").parse().unwrap();
        let r = resolve(&cfg, None, "herdr", Seen::default());
        let q = r.pal.queue;
        assert!(is_magenta(q), "{name}: {q:?} is a magenta");
        for k in QUEUE_BESIDE { assert!(apart(q, r.pal.get(k)) >= QUEUE_APART, "{name}: queue and {k} are {} apart", apart(q, r.pal.get(k)).round()); }
        if let (Some(x), Some(g)) = (q.real(), r.ground.real()) { assert!(contrast(x, g) >= LEAST, "{name}: the magenta is seen on the ground"); }
    }
    // With the terminal's own colours it is still one of the board's magentas: the sixteen's own magenta is the automations'.
    let t = resolve(&"[theme]\nname = \"terminal\"\n".parse().unwrap(), None, "herdr", Seen::default()).pal;
    assert!(is_magenta(t.queue) && t.queue != t.auto);
    // As docs/colours.md has them: herdr's default and the terminal's own colours.
    assert_eq!((resolve(&toml::Table::new(), None, "herdr", Seen::default()).pal.queue, t.queue), (sw::board::theme::C::Rgb(0xe056fd), sw::board::theme::C::Rgb(0xd946ef)));
}
