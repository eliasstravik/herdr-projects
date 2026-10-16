//! What herdr's sidebar says about a task: its board row again, in plain words. The number and title, who is on it and
//! where it stands, and the row's numbers on the right: its percent (or place in the queue) and its time to go, bare
//! (`8% 16m`, never "left"). Not the row's second line: what the agent says it does is the board's, not the sidebar's.
//! Nothing here has words of its own: they are the board row's own (shown/row_says.rs), the one list's
//! (status_words.rs), and the row's tag (store/keys.rs).
use crate::json::*;
use crate::research::is_research;
use crate::shown::row_says::row_fields;
use crate::shown::states::{as_row, who_stands};
use crate::shown::words::left;
use crate::status_words::{coordinator_status, who, COORDINATOR};
use crate::store::keys::{repo_keys, tag_of};
use crate::store::settings::values::agent_of;

/// herdr keeps 80 characters of a name or a token (checked on 0.9.3): what is longer is cut here, with "…".
pub const CAP: usize = 80;
const SEP: &str = " · ";
fn words(s: &str) -> String { crate::text::squash(&crate::store::failures::strip_ansi(s)) }
fn cut(s: &str) -> String { let all: Vec<char> = s.chars().collect(); if all.len() <= CAP { s.to_string() } else { format!("{}…", all[..CAP - 1].iter().collect::<String>().trim_end()) } }
/// A task's number and title as its row has them: `#152 Show automations…`. The number comes first and whole.
pub fn task_name(t: &Obj) -> String {
    let tag = { let g = tag_of(t.s("id"), &repo_keys()); if g.is_empty() { t.t("id") } else { g } };
    cut(&[tag, words(t.s("title"))].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" "))
}
/// One entry: rows 1, 2 and 3 of a task in herdr's sidebar. `line` is only ever the Coordinator's: a task's is empty.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Entry { pub name: String, #[serde(rename = "where")] pub place: String, pub stands: String, pub age: String, pub line: String, pub progress: String }
/// `t`: a task as the board shows it. `who_is`: whose entry it is, where that is not the task's agent.
pub fn entry_of(t: &Obj, who_is: &str, now: i64) -> Entry {
    let r = as_row(t);
    let (at_who, at_stand) = who_stands(&r);
    let f = row_fields(&r, now);
    let tags = [is_research(&r).then_some("[research]"), r.at("automation").b("id").then_some("[auto]")].into_iter().flatten().collect::<Vec<_>>().join(" ");
    // An agent that took over from another is named: `agent codex · working`, `reviewer pi · under review`.
    let whose = if !who_is.is_empty() { who_is.to_string() } else if at_who == who::REVIEWER { who::AGENT.into() } else { at_who.clone() };
    let took = if whose == who::REVIEWER { agent_of(t.s("reviewerOn")).kind } else if whose == who::AGENT && t.b("on") { t.t("agent") } else { String::new() };
    let first = [whose, took].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
    let place = [first, at_stand].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(SEP);
    let tagsets = [f.review.text(), f.pr.text()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
    let stands = [place.clone(), tagsets, tags].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(SEP);
    // The row's numbers, with no word around them: `16m` is the time to go, never `16m left`.
    let eta = if f.eta.text().is_empty() { String::new() } else { left(r.i("eta")) };
    // The row's time, by the minute: herdr shows what it was last told, so seconds would be a write a second.
    let age = if r.b("at") { format!("{}{}", if f.elapsed.ends_with('s') { "<1m".to_string() } else { f.elapsed.clone() }, if f.ago { " ago" } else { "" }) } else { String::new() };
    Entry { name: task_name(t), place, stands: cut(&words(&stands)), age, line: String::new(), progress: [f.percent.text(), eta].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ") }
}
pub fn reviewer_entry(t: &Obj, now: i64) -> Entry { entry_of(t, who::REVIEWER, now) }
/// The Coordinator's: its name and where it stands as the line above the board's box says it, and its last message.
pub fn coordinator_entry(state_word: &str, waiting: usize, said: &str) -> Entry { named_coordinator_entry(COORDINATOR, state_word, waiting, said) }
/// Any coordinator's, under its own name ("vm-str coordinator").
pub fn named_coordinator_entry(name: &str, state_word: &str, waiting: usize, said: &str) -> Entry {
    let place = format!("{name}{SEP}{}", coordinator_status(state_word, waiting));
    Entry { name: name.into(), stands: cut(&place), place, age: String::new(), line: cut(&words(said)), progress: String::new() }
}
