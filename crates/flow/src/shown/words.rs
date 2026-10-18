//! Small words and numbers the board's parts share: a time as "12m", a pull request as "PR#16" (a merge request "MR!16"), a long text cut short.
use super::ink::{Ink, Inked};
use crate::json::*;
use chrono::{Datelike, Local, TimeZone, Timelike};

pub fn ordinal(n: i64) -> String {
    let suffix = if (11..=13).contains(&(n % 100)) { "th" } else { ["th", "st", "nd", "rd"].get((n % 10) as usize).copied().unwrap_or("th") };
    format!("{n}{suffix}")
}
/// A long text's beginning, for the panel beside the list: cut between words, a few lines at most.
pub fn brief(text: &str, max: usize, mark: &str) -> String {
    let mut s = text.trim().to_string();
    while s.contains("\n\n") { s = s.replace("\n\n", "\n"); }
    let lines: Vec<&str> = s.split('\n').collect();
    if lines.len() > 8 { s = lines[..8].join("\n"); if s.chars().count() <= max { return s + mark; } }
    if s.chars().count() <= max { return s; }
    let cut: String = s.chars().take(max).collect();
    let at = cut.rfind(' ').max(cut.rfind('\n')).unwrap_or(0).max(cut.char_indices().nth(max.saturating_sub(40)).map(|(i, _)| i).unwrap_or(0));
    format!("{}{mark}", cut[..at].trim_end())
}
pub fn clean(s: &str) -> String { s.chars().map(|c| if (c as u32) < 0x20 || c as u32 == 0x7f { ' ' } else { c }).collect() }
/// A span of time in its largest unit: "12s", "4m", "3h", "2d".
pub fn age(ms: i64) -> String {
    let x = ((ms.max(0) as f64) / 1000.0).round() as i64;
    if x < 60 { format!("{x}s") } else if x < 3600 { format!("{}m", x / 60) } else if x < 86400 { format!("{}h", x / 3600) } else { format!("{}d", x / 86400) }
}
/// The minutes left on a row, never wider than four cells ("999m"): from a thousand on they are hours ("17h").
pub fn left(min: i64) -> String { if min < 1000 { format!("{min}m") } else { format!("{}h", ((min as f64 / 60.0).round() as i64).min(999)) } }
pub fn pr_number(r: &Obj) -> String { if r.b("pr") { format!("{}{}", if crate::flow::gitlab_words::is_mr(r.s("pr")) { "MR!" } else { "PR#" }, r.t("pr").rsplit('/').next().unwrap_or("")) } else { String::new() } }
pub fn merged(r: &Obj) -> bool { r.b("prMerged") || r.s("prState") == "merged" }
/// The work landed: its pull request is merged, or the task finished.
pub fn landed(r: &Obj) -> bool { merged(r) || r.s("stage") == "done" }
/// A pull request's checks are the colour of its number: green, they pass; red, they fail; plain, no result yet.
/// Merged, it goes quiet: nothing there needs your eye.
pub fn pr_color(r: &Obj) -> Ink { if merged(r) { Ink::Muted } else { match r.s("prState") { "passing" => Ink::Green, "failing" => Ink::Red, _ => Ink::Soft } } }
pub fn pr_tag(r: &Obj) -> Inked { Inked::of(pr_number(r), pr_color(r)) }
pub fn repo_label(name: &str) -> String {
    let home = crate::store::files::user_home().display().to_string();
    if crate::store::repos::known_repos().get(name).map(text) == Some(home) { "~".into() } else { name.to_string() }
}
pub fn id_label(id: &str) -> String { let mut parts = id.splitn(2, '/'); let repo = parts.next().unwrap_or(""); match parts.next() { Some(rest) => format!("{}/{rest}", repo_label(repo)), None => repo_label(repo) } }
/// A moment as a person reads it, the same on every machine: "Mon 19 Oct, 09:00".
pub fn when(ms: i64) -> String {
    let Some(d) = Local.timestamp_millis_opt(ms).single() else { return String::new() };
    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{} {} {}, {:02}:{:02}", DAYS[d.weekday().num_days_from_sunday() as usize], d.day(), MONTHS[d.month0() as usize], d.hour(), d.minute())
}
pub fn tokens(n: f64) -> String { if n >= 1e6 { format!("{:.1}M", n / 1e6) } else if n >= 1000.0 { format!("{}k", (n / 1000.0).round() as i64) } else { format!("{}", n as i64) } }
/// A progress bar of `n` cells.
pub fn bar(pct: f64, n: usize) -> String {
    let full = ((pct / 100.0) * n as f64).round().clamp(0.0, n as f64) as usize;
    format!("{}{}", "█".repeat(full), "░".repeat(n - full))
}
