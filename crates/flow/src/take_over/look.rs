//! `herdr-projects doctor` (and the end of `install`): older installs looked at again, whatever is left of them taken
//! over now, and where things stand said in a few lines, each marked when it is a problem for you to fix.
use super::{take_over, Sys};
use crate::json::*;
use crate::store::files::read_obj;
use crate::store::move_home::{short, swap_now, Env};
use crate::store::old_installs::{legacy, superwork, Places};
use std::path::Path;

fn day(ms: i64) -> String { crate::clock::iso(ms).get(..10).unwrap_or("").to_string() }
pub fn look(p: &Places, env: &Env, sys: &dyn Sys) -> Vec<String> { looked(p, env, sys).into_iter().map(|(l, _)| l).collect() }
/// The lines, each with whether it is a problem (what is left for you to do) or only what was done or found.
pub fn looked(p: &Places, env: &Env, sys: &dyn Sys) -> Vec<(String, bool)> {
    let r = take_over(p, env, sys, true, 3000, &swap_now);
    let rec = read_obj(&superwork::record_file(p)).unwrap_or_default();
    let s = |x: &str| short(Path::new(x), &p.home);
    let mut out: Vec<(String, bool)> = r.said.iter().map(|l| (l.clone(), false)).collect();
    for (what, v) in rec.o("legacy").cloned().unwrap_or_default() {
        out.push((format!("Old herdr-projects (0.2): its {what} {} in {}, set aside on {}.", if what == "projects" || what == "settings" { "are" } else { "is" }, s(v.s("to")), day(v.i("at"))), false));
    }
    if rec.o("legacy").map(|l| l.contains_key("projects")).unwrap_or(false) { out.push((format!("  The old version finds its projects there with HERDR_PROJECTS_ROOT={}.", s(rec.at("legacy").at("projects").s("to"))), false)); }
    if env.get("HERDR_PROJECTS_ROOT").map(|v| !v.is_empty()).unwrap_or(false) { out.push(("HERDR_PROJECTS_ROOT is set: only the old herdr-projects (0.2) read it. Take it out of your shell's start-up file.".into(), true)); }
    let home = rec.at("home");
    if home.s("state") == "done" {
        out.push((format!("herdr-superwork: moved into {} on {}; {} is a link to it. Remove the link once nothing started before then still runs.", s(home.s("to")), day(home.i("at")), s(home.s("from"))), false));
    }
    out.extend(r.left.iter().map(|l| (l.clone(), true)));
    if superwork::due(p) && r.stay.is_some() { out.push((format!("{} is still to be moved: another run is moving it, or the old projects are in the way. Run herdr-projects doctor again.", short(&p.superwork, &p.home)), true)); }
    if legacy::due(p) && r.left.is_empty() { out.push((format!("{} still holds something of the old herdr-projects (0.2).", short(&p.to, &p.home)), true)); }
    if out.is_empty() { out.push((NONE.into(), false)); }
    out
}
/// What is said when no older install is left.
pub const NONE: &str = "No older install of herdr-projects or herdr-superwork on this machine.";
/// For this machine, as the program runs on it.
pub fn look_here() -> Vec<String> { looked_here().into_iter().map(|(l, _)| l).collect() }
pub fn looked_here() -> Vec<(String, bool)> {
    use crate::store::move_home::{env_now, move_note, move_told};
    let (home, env) = (crate::store::files::user_home(), env_now());
    // What this run's start did, when it was not said on the terminal already.
    let note = move_note(&home, &env);
    move_told(&home, &env);
    let mut out: Vec<(String, bool)> = Some(note).filter(|n| !n.is_empty()).map(|n| (n, false)).into_iter().collect();
    out.extend(looked(&crate::store::old_installs::places(&home, &env), &env, &super::live::Live));
    out
}
