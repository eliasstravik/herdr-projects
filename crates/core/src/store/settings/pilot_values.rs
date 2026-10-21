//! What autopilot's own kinds of value may be (`Ty::Pilot`): the board events that wake a coordinator
//! (`autopilot.when`), its awake hours (`autopilot.hours`) and the task settings it may give the tasks it adds
//! (`autopilot.mayChange`). Each is checked as it is set and kept in one plain spelling; the hours are also read here.
use crate::err::{Error, Res};
use crate::json::*;
use crate::store::settings_list::{work, Entry};
use chrono::{Datelike, Local, TimeZone, Timelike};

fn refuse<T>(msg: String) -> Res<T> { Err(Error::refused(msg)) }
/// The events, in the order they are kept and said.
pub const EVENTS: [&str; 3] = ["done", "stuck", "added"];
pub const DAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
/// Areas whose settings autopilot may never give a task.
pub const NEVER: [&str; 3] = ["merge.", "limit.", "autopilot."];
fn words(v: &Value) -> Vec<String> {
    let s = match v { Value::Array(a) => a.iter().map(text).collect::<Vec<_>>().join(" "), _ => text(v) };
    s.split(|c: char| c.is_whitespace() || c == ',').filter(|w| !w.is_empty()).map(|w| w.to_lowercase()).collect()
}
pub fn check(e: &Entry, v: &Value) -> Res<Value> {
    let name = e.name.as_str();
    let w = words(v);
    match name {
        "autopilot.when" => {
            if w.len() == 1 && w[0] == "none" { return Ok(json!("none")); }
            if w.is_empty() { return refuse(format!("{name} is one or more of done, stuck, added, or none.")); }
            if let Some(bad) = w.iter().find(|x| !EVENTS.contains(&x.as_str())) { return refuse(format!("{name}: \"{bad}\" is not an event autopilot wakes on. One or more of: done, stuck, added; or none.")); }
            Ok(json!(EVENTS.iter().filter(|x| w.iter().any(|y| y == *x)).copied().collect::<Vec<_>>().join(" ")))
        }
        "autopilot.hours" => {
            let s = text(v).trim().to_lowercase();
            if s == "any" { return Ok(json!("any")); }
            hours_of(&s).map(|h| json!(h.text)).map_err(|why| Error::refused(format!("{name}: {why}")))
        }
        "autopilot.mayChange" => {
            if w.len() == 1 && w[0] == "none" { return Ok(json!("none")); }
            if w.is_empty() { return refuse(format!("{name} is a list of task settings (worker.effort) and areas (review.*), or none.")); }
            let all = work();
            let mut out: Vec<String> = vec![];
            for x in words(v).iter().map(|x| x.trim().to_string()) {
                let low = x.to_lowercase();
                if low == "*" { return refuse(format!("{name}: a bare * would let autopilot change every setting of its tasks. Name the settings (worker.effort) or areas (review.*).")); }
                if let Some(n) = NEVER.iter().find(|n| low.starts_with(*n) || format!("{low}.") == **n) { return refuse(format!("{name}: autopilot may never set {n}* on its tasks.")); }
                let ok = match low.strip_suffix(".*") { Some(area) => all.iter().any(|n| n.starts_with(&format!("{area}."))), None => all.iter().any(|n| n.to_lowercase() == low) };
                if !ok {
                    let near = crate::store::settings::values::nearest(&x, &all).map(|n| format!(" Did you mean {n}?")).unwrap_or_default();
                    return refuse(format!("{name}: \"{x}\" is no task setting nor area of them.{near}"));
                }
                let kept = all.iter().find(|n| n.to_lowercase() == low).map(|n| n.to_string()).unwrap_or(low);
                if !out.contains(&kept) { out.push(kept); }
            }
            Ok(json!(out.join(" ")))
        }
        _ => refuse(format!("{name}: no check for this kind of value.")),
    }
}
/// May a coordinator with `may` (`autopilot.mayChange`) give its task the setting `name`?
pub fn may_change(may: &str, name: &str) -> bool {
    if NEVER.iter().any(|n| name.starts_with(n)) { return false; }
    may.split_whitespace().any(|m| m != "none" && (m == name || m.strip_suffix(".*").map(|a| name.starts_with(&format!("{a}."))).unwrap_or(false)))
}

/// Awake hours as read: the days (0 Monday … 6 Sunday) and the minutes of the day from and to (to before from: across
/// midnight, the night belongs to the day it began on).
pub struct Hours { pub days: Vec<u32>, pub from: u32, pub to: u32, pub text: String }
fn clock(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    if h.is_empty() || h.len() > 2 || m.len() != 2 || !h.bytes().chain(m.bytes()).all(|b| b.is_ascii_digit()) { return None; }
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h <= 24 && m < 60 && h * 60 + m <= 1440).then_some(h * 60 + m)
}
fn day(s: &str) -> Option<u32> { DAYS.iter().position(|d| *d == s).map(|n| n as u32) }
pub fn hours_of(s: &str) -> Result<Hours, String> {
    let how = "any, or days and hours such as mon-fri 09:00-18:00, 22:00-06:00 or sat,sun 10:00-16:00";
    let parts: Vec<&str> = s.split_whitespace().collect();
    let (days_part, time) = match parts.as_slice() { [t] => (None, *t), [d, t] => (Some(*d), *t), _ => return Err(format!("\"{s}\" is not awake hours: {how}")) };
    let mut days: Vec<u32> = vec![];
    if let Some(d) = days_part {
        for piece in d.split(',') {
            match piece.split_once('-') {
                Some((a, b)) => { let (Some(a), Some(b)) = (day(a), day(b)) else { return Err(format!("\"{piece}\" is not days (mon … sun): {how}")) }; let mut x = a; loop { days.push(x); if x == b { break; } x = (x + 1) % 7; } }
                None => match day(piece) { Some(x) => days.push(x), None => return Err(format!("\"{piece}\" is not a day (mon … sun): {how}")) },
            }
        }
    } else { days = (0..7).collect(); }
    let Some((a, b)) = time.split_once('-') else { return Err(format!("\"{time}\" is not hours from and to (09:00-18:00): {how}")) };
    let (Some(from), Some(to)) = (clock(a), clock(b)) else { return Err(format!("\"{time}\" is not hours from and to (09:00-18:00): {how}")) };
    if from == to { return Err(format!("\"{time}\" begins where it ends: {how}")); }
    days.sort(); days.dedup();
    let text = format!("{}{}", days_part.map(|d| format!("{d} ")).unwrap_or_default(), time);
    Ok(Hours { days, from, to, text })
}
/// Is `at` (ms) within the awake hours `hours` (`any` always is)? This machine's time zone.
pub fn awake(hours: &str, at: i64) -> bool {
    if hours.is_empty() || hours == "any" { return true; }
    let Ok(h) = hours_of(hours) else { return true };
    let Some(t) = Local.timestamp_millis_opt(at).single() else { return true };
    let (wd, min) = (t.weekday().num_days_from_monday(), t.hour() * 60 + t.minute());
    if h.from < h.to { return h.days.contains(&wd) && min >= h.from && min < h.to; }
    // Across midnight: from the evening of an awake day to the morning after it.
    (h.days.contains(&wd) && min >= h.from) || (h.days.contains(&((wd + 6) % 7)) && min < h.to)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn may_change_takes_names_and_areas() {
        assert!(may_change("worker.effort review.*", "review.rounds"));
        assert!(may_change("worker.effort", "worker.effort"));
        assert!(!may_change("worker.effort", "worker.agent"));
        assert!(!may_change("none", "worker.effort"));
        assert!(hours_of("mon-fri 09:00-18:00").unwrap().days == vec![0, 1, 2, 3, 4]);
        assert!(hours_of("fri-mon 22:00-06:00").unwrap().days == vec![0, 4, 5, 6]);
        assert!(hours_of("9-5").is_err());
    }
}
