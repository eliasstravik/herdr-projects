//! An automation's trigger, one of two kinds: a schedule, or something that happened on the board. Reading one as it is
//! typed, saying it in plain words, and working out when a schedule is next due (this machine's time zone).
//!   { kind: "schedule", text, every: ms }   every 2h
//!   { kind: "schedule", text, cron }        daily 09:00 · mon 09:00 · cron "0 9 * * 1"
//!   { kind: "event", event, everyRepo, if? }   one of EVENTS, for the automation's repo or for every repo; `if`: a
//!                                             condition on the task (automations/condition.rs)
use super::condition::say_condition;
use super::cron::{cron_next, parse_cron, DAYS};
use super::events::{event_named, EVENTS};
use crate::err::Res;
use crate::json::*;
use crate::store::settings::values::{ms, nearest};

/// The shortest `every`: the board looks once a second, and a task a minute is already more than anyone wants.
pub const EVERY_MIN: i64 = 60000;
const HOW: &str = "A schedule is one of: every 2h · daily 09:00 · mon 09:00 (days: mon,tue,wed,thu,fri,sat,sun) · cron \"0 9 * * 1\"";
fn clock(text: &str) -> Option<(u32, u32, String)> {
    let (h, m) = text.split_once(':')?;
    if h.is_empty() || h.len() > 2 || m.len() != 2 || !h.bytes().chain(m.bytes()).all(|b| b.is_ascii_digit()) { return None; }
    let (hh, mm): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (hh < 24 && mm < 60).then(|| (hh, mm, format!("{:0>2}:{m}", h)))
}
pub fn parse_schedule(typed: &str) -> Res<Value> {
    let text = crate::text::squash(typed);
    let low = text.to_lowercase();
    let parts: Vec<&str> = low.split(' ').collect();
    let (word, rest) = (parts[0], &parts[1..]);
    if word == "cron" { let c = parse_cron(text[4..].trim().trim_matches(|c| c == '"' || c == '\''))?; return Ok(json!({ "kind": "schedule", "text": format!("cron {}", c.text), "cron": c.text })); }
    if word == "every" && rest.len() == 1 {
        let every = ms(rest[0]);
        if every == 0 { crate::fail!("\"{}\" is not a time with a unit, such as 30m or 2h (s, m or h).{}", rest[0], if rest[0].bytes().all(|b| b.is_ascii_digit()) { format!(" Did you mean {}m?", rest[0]) } else { String::new() }); }
        if every < EVERY_MIN { crate::fail!("An automation runs once a minute at most: every 1m or longer."); }
        return Ok(json!({ "kind": "schedule", "text": format!("every {}", rest[0]), "every": every }));
    }
    let at = if rest.len() == 1 { clock(rest[0]) } else { None };
    if let Some((h, m, t)) = &at {
        if word == "daily" { return Ok(json!({ "kind": "schedule", "text": format!("daily {t}"), "cron": format!("{m} {h} * * *") })); }
        let days: Vec<&str> = word.split(',').collect();
        if days.iter().all(|d| DAYS.contains(d)) {
            let nums: Vec<String> = days.iter().map(|d| DAYS.iter().position(|x| x == d).unwrap().to_string()).collect();
            return Ok(json!({ "kind": "schedule", "text": format!("{} {t}", days.join(",")), "cron": format!("{m} {h} * * {}", nums.join(",")) }));
        }
    }
    crate::fail!("\"{text}\" is not a schedule. {HOW}")
}
pub fn parse_event(typed: &str, every_repo: bool) -> Res<Value> {
    let Some(e) = event_named(typed) else {
        let ids: Vec<&str> = EVENTS.iter().map(|x| x.id).collect();
        let near = nearest(&typed.trim().to_lowercase(), &ids);
        crate::fail!("\"{typed}\" is not something that happens on the board.{} One of: {}.", near.map(|n| format!(" Did you mean {n}?")).unwrap_or_default(), EVENTS.iter().map(|x| format!("{} ({})", x.id, x.words)).collect::<Vec<_>>().join(", "));
    };
    Ok(json!({ "kind": "event", "event": e.id, "everyRepo": every_repo }))
}
/// The one trigger of a command: --schedule <words>, or --when <event> [--every-repo]. None when none is said.
pub fn trigger_of(flags: &Obj) -> Res<Option<Value>> {
    let said: Vec<&str> = ["schedule", "when"].into_iter().filter(|k| flags.contains_key(*k) && flags.at(k) != &json!(false)).collect();
    if said.len() > 1 { crate::fail!("An automation has one trigger: --schedule or --when, not both."); }
    let Some(k) = said.first() else { return Ok(None) };
    if flags.at(k) == &json!(true) {
        if *k == "schedule" { crate::fail!("--schedule needs its words. {HOW}"); }
        crate::fail!("--when needs what happened: {}", EVENTS.iter().map(|x| x.id).collect::<Vec<_>>().join(", "));
    }
    if *k == "schedule" { parse_schedule(flags.s("schedule")).map(Some) } else { parse_event(flags.s("when"), flags.at("every-repo") == &json!(true)).map(Some) }
}
/// In plain words, as the board's row and `show` say it.
pub fn say_trigger(t: &Value) -> String {
    match t.s("kind") {
        "schedule" => t.t("text"),
        "event" => format!("when a task {}{}{}", event_named(t.s("event")).map(|e| e.words).unwrap_or(t.s("event")), if t.b("everyRepo") { ", in every repo" } else { "" },
            if t.at("if").b("field") { format!(", {}", say_condition(t.at("if"))) } else { String::new() }),
        _ => "no trigger".into(),
    }
}
/// When a schedule is next due after `after`. `since`: when the trigger was set, which is what `every` counts from.
pub fn next_after(t: &Value, after: i64, since: i64) -> i64 {
    if t.s("kind") != "schedule" { return 0; }
    if t.b("every") { let every = t.i("every"); return since + (((after - since).max(0)) / every + 1) * every; }
    parse_cron(t.s("cron")).map(|c| cron_next(&c, after)).unwrap_or(0)
}
