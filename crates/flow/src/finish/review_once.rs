//! One review: the reviewer started in its own tab, its brief typed in once it is up, then waited for until its
//! verdict file appears (or its turn ends without one), and what it changed put back.
use super::review::{close_reviewer, Reviewed};
use super::review_brief::{parse_verdict, review_brief, Brief};
use super::review_fallback::{reviewer_now, Cut};
use super::shared::ms;
use crate::agents::{adapter, helpers::still_at_work, installed, start, start_failed};
use crate::clock::now;
use crate::delegate::{launch_extras, rules::task_line_as};
use crate::err::Res;
use crate::flow;
use crate::gate;
use crate::json::*;
use crate::runner::{is_aborted, Guard};
use crate::store::files::{artifacts_dir, board_env};
use crate::store::handovers::{handover_line, last_handover, log_handover};
use crate::store::marks::pending_events;
use crate::store::repos::{cwd_of, folder_of, git_out};
use crate::store::reviews::{keep_saved_review, review_history, review_round};
use crate::store::settings::values::agent_of;
use crate::store::tasks::{get, update_wait, Change};
use crate::turn::why_stopped;
use std::path::{Path, PathBuf};

/// The look at a plan, not at work (plan/review.rs): what the reviewer is told. It is no round of the task's reviews.
pub struct Plan { pub brief: Box<dyn Fn(&Obj, &Path) -> String> }
fn sleep(ms: i64) { std::thread::sleep(std::time::Duration::from_millis(ms.max(0) as u64)); }
#[allow(clippy::too_many_arguments)]
pub fn review_once(id: &str, g: &mut Guard, text: &str, change: Option<&Value>, to_land: bool, ch: &Obj, check: &str, plan: Option<&Plan>) -> Res<Reviewed> {
    // One that just took the review over says so on the row until the Watcher has a line of it.
    let mut t = g.set(|t| {
        let took = last_handover(t, "reviewer");
        let keep = took.map(|h| t.s("msg") == handover_line(&h)).unwrap_or(false);
        if !keep { t.put("msg", if plan.is_some() { "The reviewer checks the plan" } else { "Under review" }); }
        t.put("stage", "reviewing"); t.put("watch", Value::Null); t.put("waiting", Value::Null); t.put("reviewAt", now());
    })?;
    g.allow(&["reviewneeds"]);
    let api = flow::task::h(&t);
    let folder = cwd_of(&t);
    let dir = flow::task::review_dir(&t);
    let verdict_file = dir.join("verdict.json");
    // The folder is emptied before the reviewer starts. It is the task's own, in the board's data folder.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    // The configured reviewer program may not be installed here: review with the working agent's kind instead, on that
    // kind's own default model and effort. Or, with a `reviewer.fallback`, the first on that list that is installed.
    let who = start::role_of("reviewer", &|n| ch.at(n).clone());
    let (now_agent, stand_in) = reviewer_now(&t, ch);
    let on = agent_of(&now_agent);
    let (mut rkind, mut rmodel, mut reffort, mut by) = (on.kind.clone(), on.model.clone(), who.effort.clone(), now_agent.clone());
    if !installed::installed(&rkind) && installed::installed(t.s("agent")) { rkind = t.t("agent"); rmodel = String::new(); reffort = String::new(); by = rkind.clone(); }
    // A stand-in for a program that is not installed is a hand-over like any other.
    if !stand_in.is_empty() { g.set(|t| { let h = log_handover(t, "reviewer", ch.s("reviewer.agent"), &now_agent, &format!("{stand_in} is not installed here")); t.put("reviewerOn", now_agent.clone()); t.put("msg", handover_line(&h)); })?; }
    let round = if plan.is_some() { 0 } else { t.i("reviewRound") + 1 };
    let other = by != ch.s("reviewer.agent");
    let note = if rkind != who.kind && !t.b("reviewerOn") && stand_in.is_empty() { Some(format!("reviewed by {rkind}: {} is not installed here", who.kind)) } else { None };
    let was = t.t("reviewSession"); // a review started again: the conversation of the try before is not this one's
    t = g.set(|t| { if plan.is_none() { keep_saved_review(t); t.put("reviewRound", round); } t.put("reviewSession", Value::Null); let f = t.obj_mut("fallbacks"); match &note { Some(n) => f.put("reviewer", n.clone()), None => f.del("reviewer") } })?;
    // From the second round of a send-back on, the reviewer checks the earlier findings instead of starting over.
    let before: Vec<Value> = if plan.is_some() { vec![] } else { review_history(&t).into_iter().filter(gate::is_tagged).collect() };
    let earlier: Option<Vec<Value>> = if t.i("reviewRounds") > 0 && !before.is_empty() { Some(before.last().unwrap().arr("findings").to_vec()) } else { None };
    let used: Vec<String> = before.iter().flat_map(|e| e.arr("findings").iter().map(|f| f.t("id")).collect::<Vec<_>>()).collect();
    let body = match plan { Some(p) => (p.brief)(&t, &dir), None => review_brief(&t, &Brief { text, change, to_land, ch, earlier: earlier.as_deref(), check }) };
    let brief = format!("{}\n\n{body}{}", task_line_as(&t, "reviewer", round), if who.instructions.is_empty() { String::new() } else { format!("\n\n{}", start::own_words(&who.instructions)) });
    let head = if to_land { change.map(|c| c.t("head")).unwrap_or_default() } else { String::new() };
    let mut pane_tab: Option<(String, String)> = None;
    let (mut error, mut review, mut auth, mut cut, mut ended): (String, Option<Obj>, bool, Option<Cut>, bool) = (String::new(), None, false, None, false);
    let run = (|| -> Res<()> {
        installed::pre_trust(&rkind, &if t.b("root") { PathBuf::from(t.s("root")) } else { folder_of(t.s("id")) }, &who.access);
        close_reviewer(id); // a tab left over from an interrupted round
        // Its own tab, never focused: the human stays where they are, and the agent's tab and the board keep their size.
        let label = if plan.is_some() { Some(flow::tabs::PLAN_REVIEW_TAB) } else { None };
        let made = flow::tabs::open_reviewer_tab(&api, &t, round, if other { &rkind } else { "" }, &folder.display().to_string(), &board_env(), label)?;
        let pane = made.pane.clone();
        pane_tab = Some((made.pane.clone(), made.tab.clone()));
        let taken: Vec<String> = api.agents().unwrap_or_default().iter().map(|a| a.t("name")).filter(|n| !n.is_empty()).collect();
        let base = if t.at("link").b("agent") { t.at("link").t("agent") } else { format!("sw-task-{}", t.t("n")) };
        let mut name: String = format!("{base}-r").chars().take(32).collect();
        if taken.contains(&name) { name = format!("{}-{:04}", name.chars().take(27).collect::<String>(), now() % 10000); }
        let started_at = now(); // a turn-end report older than this is from a try of this review that was cut off
        let x = launch_extras(&t, "reviewer", &rkind, round, "", None)?;
        t = g.set(|t| { t.obj_mut("link").put("reviewer", json!({ "pane": pane, "tab": made.tab, "agent": name, "kind": rkind })); t.put("reviewSession", x.session.clone().map(Value::String).unwrap_or(Value::Null)); })?;
        flow::sidebar::tell_sidebar(&t); // the reviewer's pane has its entry in herdr's sidebar from the start
        let opts = |hook: Vec<String>| start::StartOpts { model: rmodel.clone(), effort: reffort.clone(), fast: Some(who.fast), access: who.access.clone(), session: x.session.clone().unwrap_or_default(), extra: hook, ..Default::default() };
        let start_with = |hook: Vec<String>| -> Res<()> {
            let args = start::start_args(&rkind, &opts(hook))?;
            let r = if plan.is_some() { crate::plan::sandbox::start_program(&api, &get(id).unwrap_or(t.clone()), &name, &rkind, &pane, &args) } else { api.agent_start(&name, &rkind, &pane, &args, 60000).map(|_| ()) };
            match r { Ok(()) => { crate::submit::settle(&api, &pane, &std::iter::once(crate::agents::kinds::program(&rkind).to_string()).chain(args).collect::<Vec<_>>().join(" ")); Ok(()) } Err(e) if e.code == "agent_not_ready" => Ok(()),
                Err(e) => Err(start_failed::say(&api, &start_failed::Start { kind: &rkind, setting: "reviewer.agent", again: "retry the review" }, &pane, &e.message, &crate::store::files::dir_of(id).join("runner.log")).into()) }
        };
        if let Err(e) = start_with(x.hook.clone()) { if x.hook.is_empty() { return Err(e); } start_with(vec![])?; } // once more without the hook arguments
        let tl = task_line_as(&t, "reviewer", round);
        let mark_cell: std::cell::RefCell<Option<Value>> = std::cell::RefCell::new(None);
        let brief_shown = || -> Option<bool> {
            let s = get(id).map(|c| c.t("reviewSession")).filter(|s| !s.is_empty()).or_else(|| adapter::find_session_but(&rkind, &folder, started_at, &tl, &was))?;
            mark_cell.borrow().as_ref().and_then(|m| crate::submit::shown(&rkind, None, &s, &folder, m, &brief))
        };
        let reads = crate::agents::kinds::kind(&rkind).map(|k| k.reads_turn).unwrap_or(false);
        let until = now() + ms("HERDR_PROJECTS_REVIEW_MS", 20 * 60000);
        let every = ms("HERDR_PROJECTS_POLL_MS", 3000);
        let (mut sent, mut sent_at, mut quiet_since, mut comp_at_send) = (false, 0i64, 0i64, Value::Null);
        let mut waited: Vec<i64> = vec![]; // its own turn-end reports while its helpers were still at work
        // Its turn ended while helpers it started are still at work (agents/helpers.rs): their answers start it again.
        let helpers_at_work = |ev: Option<&Value>| { let w = why_stopped(&get(id).unwrap_or(t.clone()), "reviewer", ev); w.turn.as_ref().map(|x| x.error.is_empty()).unwrap_or(false) && still_at_work(&w.kind, w.turn.as_ref(), started_at) };
        // The reviewer ended its turn with no verdict. Its saved conversation says why.
        let mut no_verdict = |ev: Option<&Value>, or: &str| -> String {
            let mut w = why_stopped(&get(id).unwrap_or(t.clone()), "reviewer", ev);
            let until = now() + ms("HERDR_PROJECTS_READ_MS", 5000);
            while ev.is_some() && w.can_read && !w.turn.as_ref().map(|x| x.finished || !x.error.is_empty()).unwrap_or(false) && now() < until { sleep(400); w = why_stopped(&get(id).unwrap_or(t.clone()), "reviewer", ev); }
            auth = w.turn.as_ref().map(|x| x.auth).unwrap_or(false);
            if let Some(x) = w.turn.as_ref().filter(|x| !x.error.is_empty()) { cut = Some(Cut { passing: x.passing, reason: x.reason.clone(), error: x.error.clone() }); return format!("the reviewer stopped: {}", x.error); }
            if !or.is_empty() { return or.to_string(); }
            format!("the reviewer finished without a verdict{}", w.turn.as_ref().filter(|x| x.finished && !x.said.is_empty()).map(|x| format!(". Its last message: {}", x.said.chars().take(1500).collect::<String>())).unwrap_or_default())
        };
        loop {
            g.check()?;
            // The verdict file appearing is the finish signal.
            if verdict_file.exists() {
                sleep(every.min(300)); // let the write complete
                match std::fs::read_to_string(&verdict_file).map_err(crate::err::Error::from).and_then(|raw| parse_verdict(&raw, &head, earlier.as_deref(), &used)) {
                    Ok(Some(rv)) => { review = Some(rv); break; }
                    Ok(None) => { let _ = std::fs::remove_file(&verdict_file); }
                    Err(e) => { error = crate::text::squash(&e.msg).chars().take(400).collect(); if error.is_empty() { error = "no valid verdict".into(); } break; }
                }
            }
            if now() > until { error = "review timed out".into(); break; }
            let cur = get(id).ok_or("the task is gone")?;
            // The reviewer's own turn-end report for this round, and still no verdict: the review failed, at once.
            let rev = pending_events(&cur).into_iter().find(|e| e.s("role") == "reviewer" && e.i("round") == round && !e.b("side") && e.i("at") >= started_at && !waited.contains(&e.i("n")));
            if let Some(r) = &rev { if r.b("session") && !cur.b("reviewSession") { let s = r.t("session"); let _ = update_wait(id, 10000, |t| { t.put("reviewSession", s.clone()); Ok(Change::Write) }); } }
            if let (Some(r), true) = (&rev, sent) {
                sleep(every.min(1500));
                if verdict_file.exists() { continue; }
                if helpers_at_work(Some(r)) { waited.push(r.i("n")); continue; }
                error = no_verdict(Some(r), ""); ended = true; break;
            }
            let a = api.agent_get(&pane).ok().flatten();
            if a.is_none() && sent { error = no_verdict(None, "the reviewer exited without a verdict"); ended = true; break; }
            if a.is_none() && api.pane_get(&pane).is_none() { error = "the reviewer's terminal was closed".into(); break; }
            // herdr status is used only to notice a reviewer that is blocked: "review needs you".
            let blocked = a.as_ref().map(|a| a.s("agent_status") == "blocked").unwrap_or(false);
            if blocked && cur.s("stage") == "reviewing" { g.set(|t| { t.put("stage", "reviewneeds"); t.put("msg", "The reviewer shows a dialog: open it and answer"); })?; g.allow(&["reviewing"]); }
            else if !blocked && cur.s("stage") == "reviewneeds" { g.set(|t| { t.put("stage", "reviewing"); t.put("msg", "Under review"); })?; g.allow(&["reviewneeds"]); }
            let at_rest = a.as_ref().map(|a| a.s("agent_status") == "idle" || a.s("agent_status") == "done").unwrap_or(false);
            if !sent && at_rest {
                match crate::submit::hand(&api, &pane, &brief) { Ok(m) => { crate::store::said_by::note_board(id, &brief); comp_at_send = a.as_ref().map(|a| a.at("completion_seq").clone()).unwrap_or(Value::Null); *mark_cell.borrow_mut() = m; sent = true; sent_at = now(); } Err(e) => { if e.code != "agent_blocked" && e.code != "agent_not_ready" { sent = true; sent_at = now(); } } }
            } else if mark_cell.borrow().is_some() && a.is_some() {
                // The brief was typed: did the reviewer start on it? If not, enter is pressed again; never typed twice.
                let mut m = mark_cell.borrow().clone().unwrap();
                let r = crate::submit::look(&api, &pane, &mut m, Some(&brief_shown));
                *mark_cell.borrow_mut() = Some(m);
                if r == "took" || r == "unknown" { *mark_cell.borrow_mut() = None; }
                else if r == "lost" { error = "the reviewer did not start: its brief sits unsent in its box".into(); break; }
            } else if sent && !reads && at_rest && a.as_ref().map(|a| a.at("completion_seq") != &comp_at_send && !a.at("completion_seq").is_null()).unwrap_or(false) && now() - sent_at > ms("HERDR_PROJECTS_ENDED_MS", 5000) {
                // A program whose conversation the board does not read: herdr saw it finish a turn since the brief, and
                // still no verdict 15 seconds later.
                if quiet_since == 0 { quiet_since = now(); }
                if now() - quiet_since > ms("HERDR_PROJECTS_QUIET_MS", 15000) && !verdict_file.exists() { error = no_verdict(None, ""); ended = true; break; }
            } else if sent && !reads && !at_rest { quiet_since = 0;
            } else if sent && a.is_some() && reads && now() - sent_at > ms("HERDR_PROJECTS_ENDED_MS", 5000) {
                // No hook: "idle" alone never fails a review. Only its saved conversation ending in a finished message or
                // an error, and still no verdict 15 seconds later, does.
                let mut session = cur.t("reviewSession");
                if session.is_empty() { if let Some(s) = adapter::find_session_but(&rkind, &folder, started_at, &tl, &was) { session = s.clone(); let _ = update_wait(id, 10000, |t| { t.put("reviewSession", s.clone()); Ok(Change::Write) }); } }
                let turn = if session.is_empty() { None } else { installed::read_turn_of(&rkind, None, &session, &folder, false) };
                match turn {
                    Some(x) if x.pos > sent_at && (x.finished || !x.error.is_empty()) && !(x.error.is_empty() && still_at_work(&rkind, Some(&x), started_at)) => {
                        if quiet_since == 0 { quiet_since = now(); }
                        if now() - quiet_since > ms("HERDR_PROJECTS_QUIET_MS", 15000) && !verdict_file.exists() { error = no_verdict(None, ""); ended = true; break; }
                    }
                    _ => quiet_since = 0,
                }
            }
            sleep(every);
        }
        Ok(())
    })();
    if let Err(e) = run {
        if is_aborted(&e) { close_reviewer(id); if let Some((p, tb)) = &pane_tab { flow::tabs::close_reviewer_tab(&api, &json!({ "tab": tb, "pane": p })); } return Ok(Reviewed { aborted: true, ..Default::default() }); }
        error = crate::text::squash(&e.msg).chars().take(400).collect();
    }
    // Put back anything the reviewer changed: what you approve is exactly what the agent committed. Not while you chat
    // with the agent: what changed may be its (flow/chat.rs), and the end of its turn decides.
    if to_land && !get(id).map(|c| crate::flow::chat::during(&c)).unwrap_or(false) && (flow::git::head_of(&folder) != head || !flow::git::dirty(&folder).is_empty()) { if git_out(&folder, &["reset", "--hard", &head]).is_ok() { let _ = git_out(&folder, &["clean", "-fd"]); } }
    crate::limits::spent::count(id, "reviewer", 10000); // its tokens are the task's, read while the task still names its conversation
    close_reviewer(id);
    if get(id).map(|c| c.s("stage") == "reviewneeds").unwrap_or(false) { if let Err(e) = g.set(|t| { t.put("stage", "reviewing"); t.put("msg", "Review complete"); }) { if is_aborted(&e) { return Ok(Reviewed { aborted: true, ..Default::default() }); } } }
    if review.is_none() && cut.is_some() {
        // Not a review: the task's count of reviews is put back, so the next try has this number.
        if let Err(e) = g.set(|t| { if plan.is_none() && t.i("reviewRound") == round { t.put("reviewRound", round - 1); } }) { if is_aborted(&e) { return Ok(Reviewed { aborted: true, ..Default::default() }); } }
        return Ok(Reviewed { cut: true, error, auth, by, cut_by: cut, ..Default::default() });
    }
    let Some(mut rv) = review else { return Ok(Reviewed { error: if error.is_empty() { "no valid verdict".into() } else { error }, auth, by, ended, ..Default::default() }) };
    gate::stamp(&mut rv, ch.s("review.mustFix"));
    rv.put("by", by.clone()); // who reviewed: the task's history keeps it
    if plan.is_some() { let _ = std::fs::remove_dir_all(&dir); rv.del("listed"); return Ok(Reviewed { review: Some(rv), by, ..Default::default() }); }
    // Move the folder to the task's artifacts and publish the evidence page.
    let art = artifacts_dir(id);
    if let Ok(rd) = std::fs::read_dir(&art) { for e in rd.flatten() { let n = e.file_name().to_string_lossy().into_owned(); if n != crate::research::REPORT && n != crate::plan::PLAN { let _ = std::fs::remove_dir_all(e.path()); let _ = std::fs::remove_file(e.path()); } } }
    let _ = std::fs::create_dir_all(&art);
    let _ = crate::util::copy_dir(&dir, &art);
    let _ = std::fs::remove_dir_all(&dir);
    let t = get(id).unwrap_or(t);
    let ctx = json!({ "id": id, "round": review_round(&t), "title": t.at("title"), "description": t.at("description"), "summary": text, "stat": change.map(|c| c.at("stat").clone()), "head": change.map(|c| c.at("head").clone()), "pr": t.at("pr") });
    let listed = rv.at("listed").clone();
    let artifacts = crate::review_page::publish_review(ch.b("review.evidence"), &art, &listed, &Value::Object(rv.clone()), &t, &ctx).unwrap_or_default();
    rv.put("artifacts", artifacts);
    rv.del("listed");
    Ok(Reviewed { review: Some(rv), by, ..Default::default() })
}
