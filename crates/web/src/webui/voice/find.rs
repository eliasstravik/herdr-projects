//! Who a spoken name means, among what the call's board shows: the Coordinator ("coordinator", "back"), a task by its
//! tag as said ("HP-52", "hp 52") or by words of its title ("the voice task"), or a machine's or a repo's coordinator by
//! its name ("the vm-str coordinator"). Never a guess: two that fit as well are asked about.
use crate::board::state::{Row, State};
use crate::board::tags::number_tag;
use crate::board::update::coords::of_head;
use crate::json::*;

/// Where the chat goes: a task or a heading (`id`, its agent or its coordinator), or the Coordinator (`id` empty).
#[derive(Clone, Debug, PartialEq)]
pub struct Found { pub id: String, pub name: String }
fn norm(s: &str) -> String { s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect() }
fn words(s: &str) -> Vec<String> { s.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() >= 3 && !["the", "task", "agent", "talk", "with", "den", "det", "till", "med", "prata", "uppgiften"].contains(w)).map(String::from).collect() }
const COORD: [&str; 6] = ["coordinator", "koordinator", "koordinatorn", "coordinatorn", "back", "tillbaka"];
pub fn find(s: &State, said: &str) -> Result<Found, String> {
    let w = words(said);
    let heads: Vec<(String, String)> = s.rows.iter().filter_map(|r| match r { Row::Head { id, .. } => of_head(s, id).map(|t| (id.clone(), t.name)), _ => None }).collect();
    // a heading's coordinator, named
    let named: Vec<&(String, String)> = heads.iter().filter(|(_, n)| { let nw = words(n); nw.iter().any(|x| !COORD.contains(&x.as_str()) && w.contains(x)) }).collect();
    let wants_coord = w.iter().any(|x| COORD.contains(&x.as_str()));
    if wants_coord && named.len() == 1 { return Ok(Found { id: named[0].0.clone(), name: named[0].1.clone() }); }
    // a task by its tag: its letters and number in what was said
    let said_n = norm(said);
    let tagged: Vec<&std::sync::Arc<Obj>> = s.items.iter().filter(|r| r.has("title") && [true, false].iter().any(|m| { let t = norm(&number_tag(r, &s.data.keys, &s.scope, *m)); !t.is_empty() && digits_end(&said_n, &t) })).collect();
    if tagged.len() == 1 { return Ok(task(s, tagged[0])); }
    if wants_coord && w.iter().all(|x| COORD.contains(&x.as_str())) { return Ok(Found { id: String::new(), name: "the Coordinator".into() }); }
    // a task by words of its title: the one with the most of them, alone
    let mut scored: Vec<(usize, &std::sync::Arc<Obj>)> = s.items.iter().filter(|r| r.has("title") && !r.b("archivedAt")).map(|r| (words(r.s("title")).iter().filter(|x| w.contains(x)).count(), r)).filter(|(n, _)| *n > 0).collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    match scored.as_slice() {
        [] if wants_coord => Ok(Found { id: String::new(), name: "the Coordinator".into() }),
        [] => Err(format!("No task fits \"{said}\". Ask the human for its tag or other words of its title.")),
        [(a, r), (b, r2), ..] if a == b => Err(format!("Both {} and {} fit \"{said}\": ask which one.", tag(s, r), tag(s, r2))),
        [(_, r), ..] => Ok(task(s, r)),
    }
}
/// The tag ends where a digit would go on: "hp52" is in "talktohp52" but not in "hp521".
fn digits_end(said: &str, tag: &str) -> bool { said.match_indices(tag).any(|(i, _)| !said[i + tag.len()..].starts_with(|c: char| c.is_ascii_digit())) }
fn tag(s: &State, r: &Obj) -> String { number_tag(r, &s.data.keys, &s.scope, true) }
fn task(s: &State, r: &Obj) -> Found { Found { id: r.t("id"), name: format!("{}'s agent ({})", tag(s, r), r.s("title")) } }
