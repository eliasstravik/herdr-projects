//! What a key started, shown at once: the moment an action's key is pressed its row and the task panel say what is
//! under way ("Starting…", "Stopping…"), whatever the job takes (a worktree, an agent starting, a process stopping,
//! another machine). The word stays until a read of the tasks that began after the job ended shows what really
//! happened; a job that did not work turns it red ("Could not stop: …"). Another key that would act on the same row
//! meanwhile does nothing but say that it is still under way, so nothing is started twice.
use super::Effect;
use crate::board::job::{Done, Job};
use crate::board::state::{Pending, Rec, Row, State};
use crate::json::*;
use crate::shown::ink::Ink;

/// How long a row says that what you did to it did not work (the message line says it until esc).
pub const FAILED_MS: i64 = 10_000;
/// The longest another machine's row waits for that machine to show what was done.
pub const FAR_MS: i64 = 15_000;

/// What a job's row says while it is under way, and what it says when it did not work; None: a job a row does not show.
pub fn words(s: &State, j: &Job) -> Option<(String, String)> {
    let pinned = |id: &str| s.items.iter().any(|r| r.s("id") == id && r.b("pinned"));
    let w = |doing: &str, verb: &str| Some((format!("{doing}…"), format!("Could not {verb}")));
    match j {
        Job::Tell { fresh: true, .. } | Job::Delegate { .. } => w("Starting", "start"),
        Job::Tell { .. } => w("Sending", "send"),
        Job::Approve { seen: Some(_), .. } => w("Merging", "merge"),
        Job::Approve { .. } => w("Approving", "approve"),
        Job::FinishAnyway { .. } => w("Finishing as it is", "finish it"),
        Job::Retry { .. } => w("Trying again", "try again"),
        Job::CarryOn { .. } | Job::SendFailure { .. } | Job::SendError { .. } | Job::SendReview { .. } => w("Sending back", "send it back"),
        Job::Stop { .. } => w("Stopping", "stop"),
        Job::Pin { id, .. } => if pinned(id) { w("Unpinning", "unpin") } else { w("Pinning", "pin") },
        Job::PinPlace { on, .. } => if *on { w("Pinning", "pin") } else { w("Unpinning", "unpin") },
        Job::Archive { .. } | Job::ArchiveProject { .. } => w("Archiving", "archive"),
        Job::Restore { .. } | Job::RestoreProject { .. } => w("Restoring", "restore"),
        Job::Clear { .. } | Job::AutoRemove { .. } => w("Removing", "remove"),
        Job::Pick { letter, .. } => w(&format!("Keeping attempt {letter}"), &format!("keep attempt {letter}")),
        Job::Describe { .. } => w("Adding to its description", "add to its description"),
        Job::OpenAgent { .. } => w("Opening", "open"),
        Job::AutoTurn { on, .. } => if *on { w("Turning on", "turn on") } else { w("Turning off", "turn off") },
        Job::AutoFire { .. } => w("Firing", "fire"),
        // A coordinator's row: started or stopped, its autopilot turned on or off, gone into.
        Job::CoordRun { on, .. } | Job::Pilot { on, .. } => if *on { w("Starting", "start") } else { w("Stopping", "stop") },
        Job::CoordOpen { .. } => w("Opening", "open"),
        Job::Far { action, req, .. } => match action.as_str() {
            "delegate" => w("Starting", "start"),
            "tell" => w("Sending", "send"),
            "approve" => if req.at("seen").as_array().map(|a| a.iter().any(|x| !x.is_null())).unwrap_or(false) { w("Merging", "merge") } else { w("Approving", "approve") },
            "finish" => w("Finishing as it is", "finish it"),
            "again" => w("Trying again", "try again"),
            "retry" | "fix" | "review" => w("Sending back", "send it back"),
            "stop" => w("Stopping", "stop"),
            "pin" => if pinned(&j.about()) { w("Unpinning", "unpin") } else { w("Pinning", "pin") },
            "pin-machine" | "pin-project" => if req.b("on") { w("Pinning", "pin") } else { w("Unpinning", "unpin") },
            "archive" | "project-archive" => w("Archiving", "archive"),
            "restore" | "project-restore" => w("Restoring", "restore"),
            "clear" | "auto-remove" => w("Removing", "remove"),
            "pick" => w(&format!("Keeping attempt {}", req.s("letter")), &format!("keep attempt {}", req.s("letter"))),
            "reopen" => w("Opening", "open"),
            "auto-turn" => if req.b("on") { w("Turning on", "turn on") } else { w("Turning off", "turn off") },
            "auto-run" => w("Firing", "fire"),
            "coordinator-start" | "autopilot-on" => w("Starting", "start"),
            "coordinator-stop" | "autopilot-off" => w("Stopping", "stop"),
            "coordinator-open" => w("Opening", "open"),
            _ => None,
        },
        _ => None,
    }
}
/// Words to an agent, or going into its pane: these never wait for what is under way on their row.
fn goes_through(j: &Job) -> bool {
    matches!(j, Job::Tell { .. } | Job::Describe { .. } | Job::OpenAgent { .. } | Job::CoordOpen { .. }) || matches!(j, Job::Far { action, .. } if action == "tell" || action == "reopen" || action == "coordinator-open")
}
/// Going into a pane changes nothing the board reads: its word goes when the job answers.
fn reads_back(j: &Job) -> bool { !matches!(j, Job::OpenAgent { .. } | Job::CoordOpen { .. }) && !matches!(j, Job::Far { action, .. } if action == "reopen" || action == "coordinator-open") }
/// What of a row says that something was done to it, for another machine's row, whose reads the board does not count.
fn mark_of(s: &State, id: &str) -> String {
    let Some(r) = s.items.iter().find(|r| r.s("id") == id).map(|r| &**r).or_else(|| s.autos.iter().find(|a| a.s("row") == id)) else { return "gone".into() };
    serde_json::to_string(&json!([r.at("stage"), r.at("state"), r.at("kind"), r.b("pinned"), r.b("archivedAt"), r.at("on"), r.at("app")])).unwrap_or_default()
}
fn on_screen(s: &State, id: &str) -> bool { s.rows.iter().any(|x| match x { Row::Rec(o) => o.s("id") == id, Row::Auto(a) => a.s("row") == id, _ => false }) }

/// The jobs a key (or a click, or a yes) just asked for: each one's row says at once what is under way. One that would
/// act on a row that still has something under way is not started.
pub fn start(s: &mut State, fx: &mut Vec<Effect>) {
    if !fx.iter().any(|e| matches!(e, Effect::Run(_))) { return; }
    let mut kept = vec![];
    let mut changed = false;
    for e in std::mem::take(fx) {
        let Effect::Run(j) = &e else { kept.push(e); continue };
        let id = j.about();
        let Some((word, fail)) = words(s, j).filter(|_| !id.is_empty()) else { kept.push(e); continue };
        let (through, kind) = (goes_through(j), j.kind());
        match s.pending.get_mut(&id).filter(|p| p.failed.is_none()) {
            // Something is under way on the row (not only a pane opening or words going out, which anything replaces).
            Some(p) if p.answered.is_none() && !(p.through && p.kind != kind) => {
                if through {
                    if p.kind == kind { p.out += 1; } // the same words again: the row waits for both
                    kept.push(e); continue; // words, or a look into the pane, go; the row keeps saying what is under way
                }
                let doing = p.word.trim_end_matches('…').to_lowercase();
                let tag = s.items.iter().find(|r| r.s("id") == id).map(|r| crate::board::tags::id_say(r, &s.data.keys, &s.scope)).unwrap_or_default();
                s.flash(format!("Still {doing}{}. One moment.", if tag.is_empty() { String::new() } else { format!(" {tag}") }), Ink::Muted);
                continue;
            }
            // Done, and waiting for the read that shows it: words or a look into the pane do not take its place.
            Some(p) if through && !p.through => { kept.push(e); continue; }
            _ => {}
        }
        // A row that is not in view (an archived task restored): the message line says it instead.
        if !on_screen(s, &id) && !id.starts_with("co:") && s.msg.is_empty() {
            let name = s.items.iter().find(|r| r.s("id") == id).map(|r| super::actions::name(s, r)).unwrap_or_else(|| id.clone());
            s.flash(format!("{} {name}…", word.trim_end_matches('…')), Ink::Muted);
        }
        let far = matches!(j, Job::Far { .. });
        s.pending.insert(id.clone(), Pending { word, fail, at: s.now, was: if far { mark_of(s, &id) } else { String::new() }, far, wait: reads_back(j), kind, through, out: 1, ..Default::default() });
        changed = true;
        kept.push(e);
    }
    *fx = kept;
    if changed { super::list::relist(s); }
}
/// A job answered: what failed says so on its row; what worked waits for the read that shows it.
pub fn answered(s: &mut State, d: &Done, fx: &mut Vec<Effect>) {
    // Only the job the row says is under way answers for it: another one on the same row (its pane opened) does not.
    let Some(p) = s.pending.get_mut(&d.about).filter(|p| p.kind == d.kind && p.answered.is_none() && p.failed.is_none()) else { return };
    if d.ok && p.out > 1 { p.out -= 1; return; }
    if !d.ok {
        let why = d.flash.as_ref().map(|f| f.0.clone()).unwrap_or_default();
        p.failed = Some(if why.is_empty() { p.fail.clone() } else { format!("{}: {why}", p.fail) });
        p.at = s.now;
        return;
    }
    if !p.wait { s.pending.remove(&d.about); return; }
    p.answered = Some(d.seen); p.answered_at = s.now;
    if p.far { fx.push(Effect::Poll(true)); } // that machine is asked at once for what it shows now
}
/// A read of this machine's tasks came in: what began after a job ended shows what the job did.
pub fn read(s: &mut State) {
    let seq = s.data.seq;
    s.pending.retain(|_, p| p.far || p.failed.is_some() || !p.answered.map(|n| seq > n).unwrap_or(false));
}
/// Another machine answered with its tasks: a row it shows changed, or its second answer since the job, is the real one.
pub fn polled(s: &mut State, label: &str) -> bool {
    let before = s.pending.len();
    // A coordinator of that machine: its answer after the job is what it shows now.
    s.pending.retain(|id, p| !(p.far && p.answered.is_some() && p.failed.is_none() && id.starts_with(&format!("co:{label}\t"))));
    let ids: Vec<String> = s.pending.iter().filter(|(id, p)| p.far && p.answered.is_some() && p.failed.is_none() && id.starts_with(&format!("{label}:"))).map(|(id, _)| id.clone()).collect();
    let n = s.pending.len();
    for id in ids {
        let now = mark_of(s, &id);
        let p = s.pending.get_mut(&id).unwrap();
        p.polls += 1;
        if p.polls >= 2 || now != p.was { s.pending.remove(&id); }
    }
    s.pending.len() != n || n != before
}
/// The beat: a failure has been said long enough; another machine that never showed what was done is believed anyway.
pub fn tick(s: &mut State) -> bool {
    let (now, n) = (s.now, s.pending.len());
    s.pending.retain(|_, p| if p.failed.is_some() { now - p.at < FAILED_MS } else { !(p.far && p.answered.is_some() && now - p.answered_at >= FAR_MS) });
    s.pending.len() != n
}
/// Esc dismissed the error: the rows it was about are their own again.
pub fn dismiss(s: &mut State) {
    let n = s.pending.len();
    s.pending.retain(|_, p| p.failed.is_none());
    if s.pending.len() != n { super::list::relist(s); }
}
fn shown(p: &Pending) -> Value { json!({ "word": p.failed.clone().unwrap_or_else(|| p.word.clone()), "failed": p.failed.is_some() }) }
/// The tasks as the list and the panel show them: one with something under way carries its words (`pending`).
pub fn overlay(s: &mut State) {
    if s.pending.is_empty() { return; }
    for r in s.items.iter_mut() { if let Some(p) = s.pending.get(r.s("id")) { let mut o = (**r).clone(); o.put("pending", shown(p)); *r = Rec::new(o); } }
}
/// The same for the automations' rows.
pub fn overlay_autos(s: &mut State) {
    if s.pending.is_empty() { return; }
    for a in s.autos.iter_mut() { if let Some(p) = s.pending.get(a.s("row")) { a.put("pending", shown(p)); } }
}
/// What a row with something under way says instead of its own words: in the working colour, red when it did not work.
pub fn said(p: &Value) -> Option<crate::shown::ink::Inked> {
    if !p.b("word") { return None; }
    Some(crate::shown::ink::Inked::of(p.t("word"), if p.b("failed") { Ink::Red } else { Ink::Yellow }))
}
