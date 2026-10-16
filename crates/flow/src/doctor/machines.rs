//! Other machines, for `doctor`: each one answers, as `herdr-projects machines` asks them (remote/check.rs), and they
//! can ask this one. Asked only when there are other machines, and never for long.
use super::Found;
use crate::json::*;

/// How long one machine is waited for.
pub const WAIT_MS: u64 = 5000;
/// What `herdr-projects machines` found, or None when there are no other machines.
pub fn ask() -> Option<Value> {
    if crate::remote::machines().is_empty() { return None; }
    Some(crate::remote::check::check(WAIT_MS))
}
/// The first thing to do a failure's account names, else the command that says more.
fn next(why: &Value) -> String {
    why.arr("detail").iter().map(text).find(|l| !l.trim().is_empty()).unwrap_or_else(|| "herdr-projects machines says more.".into())
}
pub fn check(r: Option<&Value>) -> Vec<Found> {
    let Some(r) = r else { return vec![Found::ok("No other machines to reach")] };
    let mut out = vec![];
    let here = r.at("here");
    if here.is_object() && !here.b("ok") { out.push(Found::bad(format!("Other machines cannot ask this one: {}", here.at("why").s("line")), next(here.at("why")))); }
    for m in r.arr("machines") {
        let (label, target) = (m.s("label"), m.s("target"));
        if !m.b("ok") { out.push(Found::bad(format!("{label} ({target}) does not answer: {}", m.at("why").s("line")), next(m.at("why")))); continue; }
        if !m.b("version") { out.push(Found::bad(format!("{label} answers, but runs an older herdr-projects"), format!("Run herdr-projects update on {label}."))); continue; }
        out.push(Found::ok(format!("{label} answers: herdr-projects {}, {} task{}", m.s("version"), m.i("tasks"), if m.i("tasks") == 1 { "" } else { "s" })));
    }
    out
}
