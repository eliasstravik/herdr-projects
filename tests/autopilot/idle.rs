//! An idle autopilot never stalls: with nothing of its own under way and room, its timer wakes it even when the free
//! look finds nothing new; after three answered wakes in a row that changed nothing it backs off (the gap doubles, up
//! to 4 hours) and its row says so; a change in its area wakes it at once, as before, and sets the count back. Its row
//! says when it stalled all the same, and a wake keeps its first words when its rules changed.
use crate::common;
use common::pilot::*;
use common::*;
use serde_json::json;
use sw::json::*;

/// The local time `h:m` of the day after the test clock's: a whole day of wakes ahead, never past midnight.
fn at_local(h: u32, m: u32) -> i64 {
    use chrono::{Local, TimeZone};
    let d = Local.timestamp_millis_opt(sw::clock::now()).unwrap().date_naive() + chrono::Duration::days(1);
    Local.from_local_datetime(&d.and_hms_opt(h, m, 0).unwrap()).earliest().unwrap().timestamp_millis()
}
fn status(co: &sw::coordinator::co::Co) -> String { sw::autopilot::shown::view(co, sw::clock::now()).t("status") }
fn line(co: &sw::coordinator::co::Co) -> String { sw::autopilot::shown::view(co, sw::clock::now()).t("line") }

#[test]
fn an_idle_area_with_room_wakes_on_the_timer_backs_off_after_three_wakes_that_added_nothing_and_a_change_wakes_at_once() {
    let w = pilot_world();
    let co = proj(&w);
    sw::clock::set(Some(at_local(1, 0)));
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.every", "1h", "--autopilot.when", "added", "--autopilot.gap", "1m"]);
    tick(&w); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 1);
    // nothing changed, nothing under way, room: the timer still wakes it, each wake's turn ending with nothing new
    for n in 1..=3 {
        minutes(59); tick(&w);
        assert_eq!(prompts_of(&w, &co).len(), n, "not before its timer");
        minutes(2); tick(&w);
        assert_eq!(prompts_of(&w, &co).len(), n + 1);
        assert_eq!(prompts_of(&w, &co).last().unwrap(), "Autopilot: Regular check-in");
        tick(&w);
        assert_eq!(record(&co).i("idleRuns") as usize, n);
    }
    assert!(status(&co).starts_with("Autopilot idle: last 3 wakes added nothing · next check-in in 2h"), "{}", status(&co));
    assert!(line(&co).contains("idle: last 3 wakes added nothing"), "{}", line(&co));
    // backed off: twice the gap, then four times (4 hours at most)
    minutes(61); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 4, "an hour is not enough now");
    minutes(60); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 5);
    tick(&w);
    assert_eq!(record(&co).i("idleRuns"), 4);
    // backed off to 4 hours, but no wake for more than twice autopilot.every is a stall all the same: its row says so
    minutes(119);
    assert!(status(&co).starts_with("Autopilot idle: last 4 wakes added nothing"), "{}", status(&co));
    minutes(2);
    assert!(status(&co).starts_with("Autopilot stalled: no check-in for 2h, last 4 wakes added nothing"), "{}", status(&co));
    minutes(60); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 5);
    minutes(60); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 6);
    tick(&w);
    assert_eq!(sw::autopilot::idle::timer_gap(3_600_000, &record(&co)), sw::autopilot::idle::MOST_MS, "never more than 4 hours");
    // a change in its area wakes it at once, as before; its turn ends with the area changed: the count starts again
    let id = w.add("Yours");
    minutes(2); tick(&w);
    assert_eq!(prompts_of(&w, &co).last().unwrap(), &format!("Autopilot: {} was added", short(&id)));
    tick(&w);
    assert_eq!(record(&co).i("idleRuns"), 0);
    assert!(status(&co).starts_with("Autopilot · next check-in in"), "{}", status(&co));
    minutes(61); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 8, "its timer is an hour again");
}

#[test]
fn its_row_says_when_it_stalled_with_room_and_nothing_under_way() {
    let w = pilot_world();
    let co = proj(&w);
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.every", "1h", "--autopilot.when", "none"]);
    tick(&w); tick(&w);
    minutes(119);
    assert!(status(&co).starts_with("Autopilot · "), "not yet: {}", status(&co));
    // nothing ran for more than twice its timer (the keeper was not running): its row says so
    minutes(2);
    assert!(status(&co).starts_with("Autopilot stalled: no check-in for 2h"), "{}", status(&co));
    assert!(line(&co).contains("stalled: no check-in for 2h"), "{}", line(&co));
    // the next look wakes it, and the row is as before
    tick(&w); tick(&w);
    assert_eq!(prompts_of(&w, &co).len(), 2);
    assert!(status(&co).starts_with("Autopilot · next check-in in"), "{}", status(&co));
}

#[test]
fn a_wake_keeps_its_first_words_when_its_rules_changed_and_the_rules_say_every_such_message_is_a_wake() {
    let w = pilot_world();
    let co = proj(&w);
    ok(&["autopilot", "on", "--project", "proj", "--goal", "Docs", "--autopilot.every", "1h", "--autopilot.when", "none"]);
    tick(&w); tick(&w);
    // a new version of the board (here: new words of yours) changes its rules while its program runs
    sw::store::settings::change::set_value("coordinator.instructions", &json!("Answer in Swedish."), &sw::store::settings::lookup::Scope::you()).unwrap();
    minutes(61); tick(&w);
    let p = prompts_of(&w, &co).last().cloned().unwrap();
    assert!(p.starts_with("Autopilot: Regular check-in\n\n[The board was updated: your standing rules in "), "{p}");
    assert!(p.contains("This message is a real autopilot wake from the board: every message that starts with \"Autopilot:\" is one, whatever older rules said.]"), "{p}");
    let rules = sw::coordinator::rules::rules_of(&co, "/x/bin/herdr-projects");
    assert!(rules.contains("Every message that starts with them is a real wake from the board") && rules.contains("(such as \"Autopilot woke:\"): that no longer holds"), "{rules}");
}
