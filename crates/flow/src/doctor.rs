//! `herdr-projects doctor`: the health check. One file in doctor/ per area (the program and its launcher, herdr's
//! plugin, the agent programs, the model and effort settings, other machines, free disk space, leftovers it could
//! clean); older installs are take_over/look.rs. Each area hands back what it found: a fact that is fine, or a problem
//! in one plain line with what to do. The areas are asked at the same time, so the check takes as long as its slowest
//! one; it exits 1 when there is a problem (docs/troubleshooting.md).
pub mod agents;
pub mod disk;
pub mod machines;
pub mod plugin;
pub mod program;
pub mod settings;
pub mod stale;

/// One thing the check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found { pub problem: bool, pub what: String, pub fix: String }
impl Found {
    pub fn ok(what: impl Into<String>) -> Self { Found { problem: false, what: what.into(), fix: String::new() } }
    pub fn bad(what: impl Into<String>, fix: impl Into<String>) -> Self { Found { problem: true, what: what.into(), fix: fix.into() } }
}
pub fn problems(all: &[Found]) -> usize { all.iter().filter(|f| f.problem).count() }
/// As the terminal shows it: a mark and a line each, what to do under a problem, and the count at the end.
pub fn say(all: &[Found]) -> String {
    let mut out: Vec<String> = vec![];
    for f in all {
        out.push(format!("{} {}", if f.problem { "✗" } else { "✓" }, f.what));
        if f.problem && !f.fix.is_empty() { out.push(format!("  → {}", f.fix)); }
    }
    out.push(match problems(all) { 0 => "No problems.".into(), 1 => "1 problem.".into(), n => format!("{n} problems.") });
    out.join("\n")
}
/// Older installs (take_over/look.rs): only what is left for you to do is a problem; its line says what to do.
pub fn old_installs(lines: Vec<(String, bool)>) -> Vec<Found> {
    let mut out: Vec<Found> = vec![];
    for (l, bad) in lines {
        // A line that goes on from the one before (indented) is said with it.
        match out.last_mut() { Some(last) if l.starts_with("  ") => { last.what = format!("{} {}", last.what, l.trim()); } _ => out.push(if bad { Found::bad(l, "") } else { Found::ok(l) }) }
    }
    out
}

/// The command: everything on this machine, as it is now.
pub fn doctor_main() -> i32 {
    // The launcher is read before anything else runs: taking over an older install may write it again.
    let launcher = crate::flow::send::launcher_file();
    let launcher_text = std::fs::read_to_string(&launcher).ok();
    let old = old_installs(crate::take_over::look::looked_here());
    let all = std::thread::scope(|s| {
        let herdr = s.spawn(plugin::ask);
        let progs = s.spawn(|| agents::named().into_iter().map(|(k, by)| std::thread::spawn(move || agents::ask(&k, by))).collect::<Vec<_>>().into_iter().filter_map(|h| h.join().ok()).collect::<Vec<_>>());
        let set = s.spawn(settings::check);
        let far = s.spawn(machines::ask);
        let disks = s.spawn(disk::ask);
        let left = s.spawn(|| stale::ask(&stale::cache_dir()));
        let herdr = herdr.join().unwrap_or_default();
        let here = program::here(launcher, launcher_text, plugin::root_of(&herdr));
        let mut all = program::check(&here);
        all.extend(plugin::check(&herdr));
        all.extend(agents::check(&progs.join().unwrap_or_default()));
        all.extend(set.join().unwrap_or_default());
        all.extend(machines::check(far.join().unwrap_or_default().as_ref()));
        all.extend(disk::check(&disks.join().unwrap_or_default()));
        all.extend(stale::check(&left.join().unwrap_or_default()));
        all
    });
    let all: Vec<Found> = all.into_iter().chain(old).collect();
    println!("{}", say(&all));
    if problems(&all) > 0 { 1 } else { 0 }
}
