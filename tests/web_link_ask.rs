//! The terminal board asks once whether the herdr web UI's link may go through its front door: a few seconds after it
//! opens, with nothing of yours open; yes points the link, no is for good, later (esc) asks the next time it opens.
use crate::common;
use common::*;
use serde_json::json;
use sw::board::update::{update, Effect, Event};
use sw::extras::{extras, note};
use sw::json::*;

const HOST: &str = "box.tail1.ts.net";
fn ticks(b: &mut common::board::Board, n: usize) -> Vec<Effect> { (0..n).flat_map(|i| update(&mut b.s, Event::Tick(1000 * i as i64))).collect() }
fn offer(b: &mut common::board::Board, urls: &[&str]) { update(&mut b.s, Event::WebUiOffer(urls.iter().map(|u| u.to_string()).collect())); }

#[test]
fn the_terminal_board_asks_once_whether_the_web_uis_link_may_go_through_the_front_door_yes_does_it_no_is_for_good_later_asks_next_time() {
    let w = world_with(Opts { repo: true, ..Default::default() });
    note(json!({ "webUi": "" }));
    // the look happens once, on the third beat, and only on the terminal board
    let mut b = common::board::open(&w.root, 30);
    assert_eq!(ticks(&mut b, 6).iter().filter(|e| **e == Effect::LookWebUi).count(), 1);
    let mut web = common::board::open(&w.root, 30); web.s.ctx.web = true;
    assert!(!ticks(&mut web, 6).contains(&Effect::LookWebUi));
    // nothing to ask about: no question
    offer(&mut b, &[]); assert!(b.s.ask.is_none());
    // a link that goes straight to the web UI: asked, with what it means; later puts it away for this board
    let url = format!("https://{HOST}");
    offer(&mut b, &[&url]);
    let a = b.s.ask.clone().expect("asked");
    assert_eq!(a.q, "Show this board in the web UI (Ovis)?");
    assert!(a.detail.contains(&format!("The web UI's link ({url}) then goes through a small front door")), "{}", a.detail);
    assert!(b.text().contains("y = yes · n = no · esc = later"), "{}", b.text());
    b.press(&["esc"]); assert!(b.s.ask.is_none()); assert_eq!(extras().s("webUi"), "");
    assert!(!ticks(&mut b, 6).contains(&Effect::LookWebUi));
    // something else is open when the answer comes: asked the next time
    let mut busy = common::board::open(&w.root, 30); ticks(&mut busy, 3);
    sw::board::update::ask::confirm(&mut busy.s, "Other?".into(), sw::board::job::Job::Nothing, sw::board::state::Valid::Always, vec![], String::new(), false);
    offer(&mut busy, &[&url]); assert_eq!(busy.s.ask.as_ref().unwrap().q, "Other?"); assert!(!busy.s.web_ui_asked);
    // no: written down, never asked again
    let mut b = common::board::open(&w.root, 30); offer(&mut b, &[&url]); b.press(&["n"]);
    assert_eq!(extras().s("webUi"), "no"); assert!(!b.text().contains("Left as it is"), "no confirmation under the chat: {}", b.text());
    assert!(sw::webui::command::offer().is_empty());
    // yes: done, and the board says how it went (here no web UI is installed, so it says that)
    note(json!({ "webUi": "" }));
    let mut b = common::board::open(&w.root, 30); offer(&mut b, &[&url]); b.press(&["y"]);
    assert!(b.s.ask.is_none());
    assert!(b.text().contains("The web UI (Ovis) is not installed"), "{}", b.text());
}
