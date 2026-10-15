//! What you do to the selected task: send what you typed, put an agent on it, yes, send back, stop, remove, undo,
//! archive, pin; who you talk to; the panel. Each records what to do as a job (job.rs) and says nothing it cannot know
//! yet: what happened is said when the job answers.
use super::ask::{confirm, merge_mark};
use super::keys::read_only;
use super::list::{relist, step};
use super::Effect;
use crate::attempts::words::{agent_word, standing_of};
use crate::board::job::Job;
use crate::board::keys::key_name;
use crate::board::state::{State, Valid};
use crate::board::tags::{name_say, number_tag};
use crate::json::*;
use crate::shown::ink::Ink;
use crate::shown::states::{failed_review, failure_for, is_agent, stuck_finishing};
use crate::shown::words::{clean, pr_number};

pub fn name(s: &State, r: &Obj) -> String { name_say(r, &s.data.keys, &s.scope, 40) }
pub fn go(_s: &mut State, j: Job, fx: &mut Vec<Effect>) { fx.push(Effect::Run(j)); }
fn far(s: &State, r: &Obj, action: &str, extra: Value, quiet: bool) -> Job {
    let m = s.remotes.get(r.s("host")).map(|m| m.machine.clone()).unwrap_or_default();
    let mut req = obj(extra); req.put("id", r.t("rid"));
    Job::Far { machine: m, host: r.t("host"), action: action.into(), req: Value::Object(req), quiet }
}
/// The same action on whichever machine has the task, said while it is on its way there.
fn act(s: &mut State, r: &Obj, action: &str, extra: Value, local: Job, fx: &mut Vec<Effect>) {
    if r.b("host") {
        let doing = match action { "pin" => if r.b("pinned") { "Unpinning" } else { "Pinning" }, "archive" => "Archiving", "restore" => "Restoring", "delegate" => "Putting an agent on", "stop" => "Stopping", "clear" => "Removing", "approve" => "Merging", "set" => "Changing a setting of", _ => "Sending to" };
        let tag = crate::board::tags::id_say(r, &s.data.keys, &s.scope);
        s.flash(format!("{doing} {tag} on {}…", r.s("host")), Ink::Muted);
        fx.push(Effect::Run(far(s, r, action, extra, false)));
    } else { fx.push(Effect::Run(local)); }
}
/// A reply to this task goes to an agent of its own (not, with its task named, to the Coordinator).
pub fn has_agent(r: &Obj) -> bool { is_agent(r) && (r.b("host") || crate::flow::task::is_agent_task(r) || crate::flow::git::cleaned_up(r) || r.s("stage") == "attempts") }

// ---- enter and tab ---------------------------------------------------------------------------------------------------
pub fn enter(s: &mut State, text: &str, r: Option<&Obj>, fx: &mut Vec<Effect>) {
    if text.is_empty() { if let Some(r) = r { open_agent(s, r, fx); } return; } // with nothing typed, enter opens the selected task's agent
    // Who gets it is who the chat names: the selected task's agent after space, the Coordinator otherwise.
    if s.reply { if let Some(r) = r {
        if r.b("archivedAt") { return s.flash(read_only(), Ink::Yellow); }
        if r.s("id") != s.reply_id { return s.flash("The task you were replying to is gone. Nothing was sent.", Ink::Yellow); }
    } }
    let words = s.value();
    let reply_to = if s.reply { r.cloned() } else { None };
    match &reply_to {
        // A reply to one of this machine's tasks that never had an agent starts one with that message.
        Some(r) if !r.b("host") && !is_agent(r) && ["todo", "progress"].contains(&r.s("stage")) => fx.push(Effect::Run(Job::Tell { id: r.t("id"), text: words.trim().to_string(), fresh: true })),
        Some(r) if has_agent(r) => if r.b("host") { fx.push(Effect::Run(far(s, r, "tell", json!({ "text": words.trim() }), false))); } else { fx.push(Effect::Run(Job::Tell { id: r.t("id"), text: words.trim().to_string(), fresh: false })); },
        _ => dispatch(s, words.trim(), false, reply_to.as_ref(), fx),
    }
    s.input.set_text("");
}
pub fn tab(s: &mut State, text: &str, r: Option<&Obj>, fx: &mut Vec<Effect>) {
    if !text.is_empty() { if (s.reply && r.is_some()) || s.mode != crate::board::state::Mode::New { return; } let w = s.value(); dispatch(s, w.trim(), true, None, fx); s.input.set_text(""); return; }
    let Some(r) = r else { return };
    if r.b("archivedAt") { return s.flash(read_only(), Ink::Yellow); }
    let n = name(s, r);
    // One that waits for other tasks to be done starts by itself: early only on a yes.
    if r.s("stage") == crate::after::STAGE {
        let job = if r.b("host") { far(s, r, "delegate", json!({ "now": true }), false) } else { Job::Delegate { id: r.t("id"), name: n.clone(), now: true } };
        let why = if r.b("afterBlocked") { format!("It waits for you: {}.", r.s("afterBlocked")) } else { format!("It {}.", crate::after::waiting_words(r).unwrap_or_default()) };
        return confirm(s, format!("Start {n} now?"), job, Valid::Always, vec![("sayYes", "start now".into()), ("sayNo", "let it wait".into())], why, false);
    }
    act(s, r, "delegate", json!({}), Job::Delegate { id: r.t("id"), name: n, now: false }, fx);
}
/// What you typed, for the Coordinator: this machine's one long-lived agent, or another machine's (for a task of
/// theirs you have open, or when the words start with its name: "@vm-a fix the build"). With no Coordinator what you
/// type is added as a task, word for word.
pub fn dispatch(s: &mut State, text: &str, delegate: bool, about: Option<&Obj>, fx: &mut Vec<Effect>) {
    let named = crate::re!(r"^@?([A-Za-z0-9_.-]+)[:\s]\s*([\s\S]+)$").captures(text).map(|c| (c[1].to_string(), c[2].to_string()));
    let at_named = named.as_ref().filter(|(m, _)| s.all() && (text.starts_with('@') || text[m.len()..].starts_with(':'))).and_then(|(m, _)| s.remotes.get(m)).map(|m| m.machine.label.clone());
    let there = match about { Some(a) if a.b("host") => Some(a.t("host")), Some(_) => at_named, None => at_named.or_else(|| s.remotes.get(&s.scope).map(|m| m.machine.label.clone())) };
    if let Some(label) = there.filter(|l| s.remotes.contains_key(l)) {
        let named_here = named.as_ref().map(|(m, _)| *m == label).unwrap_or(false);
        let words = if about.map(|a| a.b("host")).unwrap_or(false) || !named_here { text.to_string() } else { named.unwrap().1 };
        let m = s.remotes[&label].machine.clone();
        fx.push(Effect::Run(Job::Dispatch { machine: m, text: words, delegate, about: about.filter(|a| a.b("host")).map(|a| a.t("rid")).unwrap_or_default() }));
        fx.push(Effect::Load);
        return;
    }
    if !s.data.can_dispatch {
        if let Some(a) = about.filter(|a| !a.b("host")) { fx.push(Effect::Run(Job::Describe { id: a.t("id"), text: text.into() })); return; }
        fx.push(Effect::Run(Job::Add { cwd: s.ctx.cwd.clone(), text: text.into(), delegate }));
        return;
    }
    fx.push(Effect::Run(Job::CoordSend { cwd: s.ctx.cwd.clone(), text: text.into(), delegate, about: about.map(|a| a.t("id")).unwrap_or_default() }));
    fx.push(Effect::Load);
}
/// Into the selected task's agent: its real herdr pane (attempts.rs: the selected attempt's while one is picked).
pub fn open_agent(s: &mut State, r: &Obj, fx: &mut Vec<Effect>) {
    if s.ctx.web { return super::web::open_agent(s, r, fx); }
    if r.b("archivedAt") { return open_panel(s); }
    if r.b("host") {
        if !r.at("link").b("pane") { return s.flash(format!("No agent has been on this yet. {} puts one on it.", key_name("delegate")), Ink::Yellow); }
        // Looked at through herdr's web UI this terminal must not become that agent's: the way back would not get through.
        if s.guest || s.viewed { return s.flash(format!("In the web UI, pick its agent in the sidebar under {}: “{}”. From here herdr's key for the way back would not get through while the web UI shows this pane.", r.s("host"), clean(if r.b("title") { r.s("title") } else { r.s("rid") })), Ink::Yellow); }
        s.stick = r.t("id");
        s.flash(format!("{}: opening…", r.s("host")), Ink::Muted);
        // In a herdr window that machine's herdr puts it in front; outside one (plain SSH) this terminal attaches.
        let mut j = far(s, r, "reopen", json!({ "pane": r.at("link").t("pane"), "focus": s.ctx.in_herdr }), true);
        if let Job::Far { req, .. } = &mut j { req.as_object_mut().unwrap().put("pane", r.at("link").t("pane")); }
        return fx.push(Effect::Run(j));
    }
    let mut target = r.clone();
    if !crate::flow::task::is_agent_task(r) { if let Some(c) = attempt_now(s, r) {
        if !crate::flow::task::is_agent_task(&c) { return s.flash(format!("Attempt {} has no terminal {}.", c.s("letter"), if c.s("stage") == "queued" { "yet: it waits for a free slot" } else { "any more" }), Ink::Yellow); }
        target = c;
    } }
    if !crate::flow::task::is_agent_task(&target) { return s.flash(format!("No agent has been on this yet. {} puts one on it.", key_name("delegate")), Ink::Yellow); }
    s.stick = r.t("id"); // whatever the task becomes while you are in there, you come back to it
    fx.push(Effect::Run(Job::OpenAgent { id: target.t("id"), popup: s.ctx.popup }));
}

// ---- the keys ----------------------------------------------------------------------------------------------------------
/// "Yes": whatever the selected task waits for. A merge cannot be undone, so that one asks first.
pub fn yes(s: &mut State, r: &Obj, fx: &mut Vec<Effect>) {
    if r.b("archivedAt") { return s.flash(read_only(), Ink::Yellow); }
    if r.s("stage") == "pick" { return pick_selected(s, r); }
    let id = r.t("id");
    let st = |x: &str| r.s("stage") == x;
    if st("approval") && r.s("state") == "blocked" {
        if r.b("syncBlocked") { return s.flash(format!("Behind main, so it cannot be approved yet: {}. The board keeps trying to bring it up to date.", r.at("syncBlocked").s("why")), Ink::Yellow); }
        let bad_review = failed_review(r);
        let tally = if bad_review { crate::status_words::review_tally(r) } else { String::new() };
        // GitHub's checks failed on what would merge (land/checks.rs): a yes is a merge anyway, and asks as one.
        let checks = crate::land::checks::failed_on(r);
        let failed = bad_review || checks.is_some();
        let onto = if r.b("host") { format!("{} on {}", if r.at("snap").b("landsOn") { r.at("snap").t("landsOn") } else { "its main branch".into() }, clean(r.s("host"))) } else if r.at("snap").b("landsOn") { r.at("snap").t("landsOn") } else { "main".into() };
        let tag = number_tag(r, &s.data.keys, &s.scope, true);
        let what = [tag, format!("\"{}\"", clean(if r.b("title") { r.s("title") } else { r.s("id") }))].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" ");
        let pr = pr_number(r);
        let seen = (r.at("pr").clone(), r.at("reviewedHead").clone());
        let job = if r.b("host") { far(s, r, "approve", json!({ "seen": [seen.0, seen.1], "anyway": failed }), false) } else { Job::Approve { id: id.clone(), seen: Some(seen), anyway: failed } };
        let mark = merge_mark(s, &id);
        let detail = format!("{}{}A merge cannot be undone.", if bad_review { format!("Its last review failed{}. ", if tally.is_empty() { String::new() } else { format!(": {tally}") }) } else { String::new() }, checks.map(|c| format!("{}{}. ", c[..1].to_uppercase(), &c[1..])).unwrap_or_default());
        return confirm(s, format!("Merge {what}{} into {onto}?", if pr.is_empty() { String::new() } else { format!(" ({pr})") }), job, Valid::Mark(id, mark),
            vec![("sayYes", if failed { "merge anyway" } else { "merge" }.into()), ("sayNo", "do not merge".into())], detail, true);
    }
    let simple = |s: &mut State, action: &str, local: Job, fx: &mut Vec<Effect>| act(s, r, action, json!({}), local, fx);
    if is_agent(r) && r.s("state") == "blocked" && r.s("kind") == "permission" { return simple(s, "approve", Job::Approve { id, seen: None, anyway: false }, fx); }
    // Stopped at a limit, without a summary, or with the board stuck at "Finishing": yes takes the work as it is,
    // through the check and the review.
    if (st("stopped") && r.b("limitStop")) || st("nosummary") || stuck_finishing(r) { return simple(s, "finish", Job::FinishAnyway { id }, fx); }
    // Something went wrong: yes tries the same step again, as it stands.
    if st("failed") || (st("asks") && r.s("kind") == "auth") { return simple(s, "again", Job::Retry { id }, fx); }
    // A plan that waits for your yes: the build starts.
    if st("plan") && r.s("state") == "blocked" { return simple(s, "approve", Job::Approve { id, seen: None, anyway: false }, fx); }
    s.flash("Nothing here waits for a yes.", Ink::Muted);
}
/// "Send it back": whatever went wrong (a failed step, a conflict, the review) goes to the task's agent to fix.
pub fn send_back(s: &mut State, r: &Obj, fx: &mut Vec<Effect>) {
    if r.b("archivedAt") { return s.flash(read_only(), Ink::Yellow); }
    let id = r.t("id");
    if r.s("stage") == "stopped" && r.b("limitStop") { return act(s, r, "retry", json!({}), Job::CarryOn { id }, fx); }
    let failure = failure_for(r);
    if let Some(f) = &failure { if !f.b("agentCanFix") && !(r.s("stage") == "failed" && r.at("failure").b("auth")) { return s.flash(format!("This needs you first. {}", f.s("next")), Ink::Yellow); } }
    let review = r.at("review");
    if failure.is_none() && r.s("state") != "error" && !(review.b("text") && crate::gate::sendable(review)) { return s.flash("Nothing to send back here.", Ink::Muted); }
    let fix = failure.as_ref().map(|f| f.b("agentCanFix")).unwrap_or(false);
    let (action, local) = if fix { ("fix", Job::SendFailure { id }) } else if r.s("state") == "error" { ("retry", Job::SendError { id }) } else { ("review", Job::SendReview { id }) };
    act(s, r, action, json!({}), local, fx);
}
/// An agent is on it right now: at work, in a step of the finish line, waiting for you to answer it, or queued (for a
/// slot, or after other tasks: stop stops the waiting).
pub fn running(r: &Obj) -> bool {
    let live = crate::store::files::SLOT_STAGES.contains(&r.s("stage")) || ["asks", "plan"].contains(&r.s("stage"));
    (r.s("state") == "working" && (is_agent(r) || r.s("stage") == "progress")) || ["queued", crate::after::STAGE].contains(&r.s("stage")) || (is_agent(r) && live)
}
/// Stop the agent while one runs. On a task with none running the same key asks to remove the task; a stop never
/// goes on to that question by itself.
pub fn stop(s: &mut State, r: &Obj, fx: &mut Vec<Effect>) {
    if !running(r) { return remove(s, r); }
    let said = format!("Stopped {}.", name(s, r));
    act(s, r, "stop", json!({}), Job::Stop { id: r.t("id"), said }, fx);
}
/// The question shows the whole title and what is really lost.
pub fn remove(s: &mut State, r: &Obj) {
    let tree = if r.b("host") { r.at("snap").t("tree") } else { r.at("snap").t("tree") };
    let lost = format!("This cannot be undone. The task goes, with its description{} and what its agent said{}. {}{}",
        if r.b("review") || r.b("reviewRound") { ", its reviews" } else { "" }, if running(r) { "; its agent is stopped" } else { "" },
        if tree.is_empty() { "It has no worktree.".to_string() } else { format!("Its worktree {tree} goes too; work in it that is not saved or not merged is kept.") },
        if r.b("archivedAt") { String::new() } else { format!(" {} only archives it.", key_name("archive")) });
    let n = name_say(r, &s.data.keys, &s.scope, usize::MAX);
    let job = if r.b("host") { far(s, r, "clear", json!({}), false) } else { Job::Clear { id: r.t("id"), name: name(s, r) } };
    confirm(s, format!("Remove {n}?"), job, Valid::Always, vec![("sayYes", "remove".into()), ("sayNo", "keep".into())], lost, true);
}
pub fn undo(s: &mut State, fx: &mut Vec<Effect>) {
    // A draft comes back only to whoever it was for: a reply stays a reply to that task, never a new task.
    if let Some(mut d) = s.draft_away.clone() {
        if !d.to.is_empty() && !s.items.iter().any(|r| r.s("id") == d.to && !r.b("archivedAt")) {
            if !d.warned { d.warned = true; s.draft_away = Some(d); return s.flash(format!("The task that reply was for is gone. {} again brings the words back as a draft of their own.", key_name("undo")), Ink::Yellow); }
            d.to.clear();
        }
        let now = if s.reply && s.current().is_some() { s.sel.clone() } else { String::new() };
        if d.to != now {
            let title = s.items.iter().find(|r| r.s("id") == d.to).map(|r| r.t("title")).unwrap_or_else(|| d.to.clone());
            let nm = format!("\"{}\"", crate::text::cut(&clean(&title), 40));
            s.draft_away = Some(d.clone());
            return s.flash(if d.to.is_empty() { format!("Those words were for the Coordinator. {} goes back to it, then {} brings them back.", key_name("talk"), key_name("undo")) }
                else if !now.is_empty() { format!("Those words were for {nm}. Move to it, then {} brings them back.", key_name("undo")) }
                else { format!("Those words were for the agent on {nm}. Select it and press {}, then {} brings them back.", key_name("talk"), key_name("undo")) }, Ink::Yellow);
        }
        s.draft_away = None;
        super::setup::away_home(s, &d);
        s.input.set_text(&d.text);
        return relist(s);
    }
    let Some(r) = s.archive_undo.take() else { return s.flash("Nothing to undo.", Ink::Muted) };
    let said = format!("Restored {}.", name(s, &r));
    act(s, &r, "restore", json!({}), Job::Restore { id: r.t("id"), said }, fx);
}
pub fn archive_selected(s: &mut State, r: &Obj, fx: &mut Vec<Effect>) {
    let archiving = !r.b("archivedAt");
    s.full = false;
    let n = name(s, r);
    let said = if archiving { format!("Archived {n}. {} undoes it; {} shows the archive.", key_name("undo"), key_name("archiveView")) } else { format!("Restored {n}.") };
    let local = if archiving { Job::Archive { id: r.t("id"), said } } else { Job::Restore { id: r.t("id"), said } };
    act(s, r, if archiving { "archive" } else { "restore" }, json!({}), local, fx);
}
pub fn pin(s: &mut State, r: &Obj, fx: &mut Vec<Effect>) {
    if r.b("archivedAt") { return s.flash(read_only(), Ink::Yellow); }
    let n = name(s, r);
    act(s, r, "pin", json!({}), Job::Pin { id: r.t("id"), name: n, key: key_name("pin") }, fx);
}
/// The one key that changes who you talk to: the selected task's agent, and back to the Coordinator.
pub fn flip_reply(s: &mut State, r: Option<&Obj>) {
    let Some(r) = r else { return };
    if s.reply { s.reply = false; s.panel_top = 0; return relist(s); }
    if r.b("archivedAt") { return s.flash(read_only(), Ink::Yellow); }
    s.reply = true; s.reply_id = r.t("id");
}
pub fn move_by(s: &mut State, d: i64) { step(s, d); }
pub fn toggle_archive(s: &mut State) {
    if !s.archive_view {
        s.archive_back = Some(crate::board::state::Back { sel: s.sel.clone(), top: s.top, filter: s.filter.clone(), mode: Some(s.mode), value: s.value() });
        s.archive_view = true; s.filter.clear(); s.mode = crate::board::state::Mode::New; s.input.set_text(""); s.full = false; s.top = 0;
    } else {
        s.archive_view = false;
        if let Some(b) = s.archive_back.take() { s.sel = b.sel; s.top = b.top; s.filter = b.filter; s.mode = b.mode.unwrap_or(crate::board::state::Mode::New); s.input.set_text(&b.value); }
        s.full = false;
    }
    relist(s);
}
pub fn toggle_side(s: &mut State, fx: &mut Vec<Effect>) {
    s.side = !s.side; fx.push(Effect::Save);
    if s.side && s.width < crate::board::view::WIDE { return s.flash(format!("The side panel needs a wider window. {} shows the task full screen.", key_name("panel")), Ink::Yellow); }
    s.flash(if s.side { format!("Side panel on. {} hides it.", key_name("side")) } else { format!("Side panel off. {} shows it.", key_name("side")) }, Ink::Muted);
}
pub fn fold_panel(s: &mut State) { s.full = false; s.editing = None; s.panel_top = 0; relist(s); }
pub fn open_panel(s: &mut State) { if s.current().is_none() && s.job().is_none() { return s.flash("Select a task first.", Ink::Muted); } s.full = true; s.panel_top = 0; }
pub fn scroll_panel(s: &mut State, by: i64) { let last = s.panel_length.saturating_sub(s.panel_height) as i64; s.panel_top = (s.panel_top as i64 + by).clamp(0, last.max(0)) as usize; }
pub fn free_size(s: &mut State, fx: &mut Vec<Effect>) {
    let by: Vec<i64> = s.held.as_ref().filter(|_| !s.guest).map(|h| h.by.iter().map(|b| b.0).collect()).unwrap_or_default();
    if by.is_empty() { return s.flash("Nothing holds this pane at another size.", Ink::Muted); }
    fx.push(Effect::Run(Job::FreeSize { pids: by }));
}

// ---- several attempts at once ----------------------------------------------------------------------------------------
pub fn tries_of(r: &Obj) -> Vec<Obj> { if r.b("archivedAt") { vec![] } else { r.arr("tries").iter().filter_map(|x| x.as_object().cloned()).collect() } }
/// Is there still a choice among them: its attempts are under way, or it waits for a pick.
pub fn choosing(_s: &State, r: &Obj) -> bool { tries_of(r).len() > 1 && !r.b("picked") && ["attempts", "pick", "reviewing"].contains(&r.s("stage")) }
/// The selected attempt's letter: the one you moved to, else the reviewer's choice, else the first that passed.
pub fn attempt_sel(s: &State, r: &Obj) -> String {
    let tries = tries_of(r);
    if let Some(at) = s.attempt_at.get(r.s("id")) { if tries.iter().any(|c| c.s("letter") == at) { return at.clone(); } }
    let passed: Vec<String> = tries.iter().filter(|c| standing_of(c, s.now) == "passed").map(|c| c.t("letter")).collect();
    let pick = r.at("comparison").t("pick");
    if passed.contains(&pick) { return pick; }
    passed.first().cloned().or_else(|| tries.first().map(|c| c.t("letter"))).unwrap_or_default()
}
pub fn attempt_now(s: &State, r: &Obj) -> Option<Obj> { let l = attempt_sel(s, r); tries_of(r).into_iter().find(|c| c.s("letter") == l) }
pub fn next_attempt(s: &mut State, r: &Obj) {
    let tries = tries_of(r);
    if tries.is_empty() { return; }
    let n = tries.iter().position(|c| c.s("letter") == attempt_sel(s, r)).unwrap_or(0);
    s.attempt_at.insert(r.t("id"), tries[(n + 1) % tries.len()].t("letter"));
}
/// Your pick. It asks first: the other attempts' tabs close with it.
pub fn pick_selected(s: &mut State, r: &Obj) {
    let Some(c) = attempt_now(s, r) else { return s.flash("This task has no attempts.", Ink::Muted) };
    let letter = c.t("letter");
    if standing_of(&c, s.now) != "passed" { return s.flash(format!("Attempt {letter} did not pass the check, so it cannot be kept. Move to one that passed."), Ink::Yellow); }
    let job = if r.b("host") { far(s, r, "pick", json!({ "letter": letter }), false) } else { Job::Pick { id: r.t("id"), letter: letter.clone() } };
    let tag = number_tag(r, &s.data.keys, &s.scope, true);
    confirm(s, format!("Keep attempt {letter} ({}) of {tag} \"{}\"?", agent_word(&c), clean(if r.b("title") { r.s("title") } else { r.s("id") })), job, Valid::Always,
        vec![("sayYes", format!("keep {letter}")), ("sayNo", "not yet".into())], "Only this one is reviewed, pushed and merged. The other attempts' tabs close; their worktrees stay until the task is done.".into(), false);
}
