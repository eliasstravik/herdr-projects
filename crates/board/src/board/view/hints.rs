//! The hint line at the bottom: what matters right now, the main thing first, at most four. Each one says what the key
//! does and to what: "this task" is the selected one. A hint is a key and its words, and may say whether it stays when
//! only one fits and shorter words for a window too narrow for the usual ones (and, for a draft, shortest ones yet: the
//! shorter words still name who gets it).
use super::chat::{chat_up, Who};
use super::textbox::{draft_task, far_coordinator};
use crate::board::keys::{hint, name, pair, K};
use crate::board::state::{Mode, State};
use crate::board::update::actions::{attempt_now, choosing};
use crate::json::*;
use crate::shown::states::{can_send_error, failed_review, failure_for, in_review, is_agent, stuck_finishing};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hint { pub key: String, pub words: String, pub stays: bool, pub short: String, pub shortest: String }
fn h(p: (String, String)) -> Hint { Hint { key: p.0, words: p.1, ..Default::default() } }
fn hk(k: K, words: &str) -> Hint { h(hint(k, Some(words))) }
fn plain(key: String, words: &str) -> Hint { Hint { key, words: words.into(), ..Default::default() } }
/// What esc goes back from right now, with nothing typed.
pub fn esc_step(s: &State) -> &'static str { if s.full || s.auto_open { "back to the list" } else if !s.filter.is_empty() { "clear the filter" } else if s.archive_view { "back to the board" } else { "" } }
/// The hints. Also says whether the chat's own key is on the line while the message is hidden (it outlasts others).
pub fn hints(s: &State, r: Option<&Obj>, who: Option<&Who>) -> (Vec<Hint>, Option<Hint>) {
    let more = h(hint(K::Menu, None));
    let (enter, esc, tab) = (name(K::Enter), name(K::Back), name(K::Delegate));
    let put = plain(esc.clone(), "put away");
    let line = h(hint(K::Newline, None));
    let arrows = { let (u, d) = (name(K::Up), name(K::Down)); if u.chars().count() + d.chars().count() > 2 { format!("{u} {d}") } else { format!("{u}{d}") } };
    let up = who.map(|w| w.said && chat_up(s, Some(w))).unwrap_or(false);
    let key: Vec<Hint> = if who.map(|w| w.said && !w.text.is_empty() && !s.chat_folded).unwrap_or(false) { vec![hk(K::ChatMessage, &format!("{} the latest message", if up { "hide" } else { "show" }))] } else { vec![] };
    let chat_key = if up { None } else { key.first().cloned() };
    if s.ask.is_some() { return (vec![], chat_key); }
    // After esc at the top (which closes nothing) the line names the key that does.
    if let Some((_, by_esc)) = s.closing {
        let w = if by_esc { format!("{} closes the board", name(K::Close)) } else { format!("Press {} again to close the board", name(K::Close)) };
        return (vec![Hint { key: String::new(), words: w, stays: false, short: if by_esc { format!("{} closes the board", name(K::Close)) } else { format!("{} again closes the board", name(K::Close)) }, ..Default::default() }], chat_key);
    }
    if s.menu.is_some() { return (vec![plain(arrows, "pick a key"), plain(enter, "press it"), plain(esc, "close the menu")], chat_key); }
    if let Some(e) = &s.editing {
        return (if !e.typing.is_empty() { vec![plain(enter, "save this value"), plain(esc, "leave it as it is")] } else { vec![plain(pair(K::Up), "move"), plain(enter, "change"), plain(name(K::PutBack), "put back"), plain(esc, "back to the task")] }, chat_key);
    }
    if s.mode == Mode::Find { return (vec![plain(enter, "keep the filter"), plain(esc, "cancel")], chat_key); }
    let agent_on = r.map(is_agent).unwrap_or(false);
    // While you type: what the keys do with your words, and who gets them. A reply names its task by its tag: the
    // words are locked to it while the list may be scrolled to others, and the box's edge names no one.
    if !s.value().is_empty() {
        if let (true, Some(r)) = (s.reply, r) {
            let (fresh, t) = (!r.b("host") && ["todo", "progress"].contains(&r.s("stage")), draft_task(s, r));
            let (w, short) = if crate::board::update::actions::has_agent(r) { (format!("send to {t}'s agent"), format!("send to {t}")) } else if fresh { (format!("send, and start an agent on {t}"), format!("send to {t}")) } else if s.data.can_dispatch || r.b("host") { (format!("send to the Coordinator, about {t}"), "send to the Coordinator".into()) } else { (format!("add to {t}'s description"), format!("add to {t}")) };
            return (vec![Hint { key: enter, words: w, stays: false, short, shortest: "send".into() }, line, put], chat_key);
        }
        let (coordinator, short) = far_coordinator(s).map(|m| (format!("send to the Coordinator on {m}"), format!("send to {m}"))).unwrap_or_else(|| ("send to the Coordinator".into(), "send to the Coordinator".into()));
        if s.talks_to_coord() { return (vec![Hint { key: enter, words: coordinator, stays: false, short, shortest: "send".into() }, plain(tab, "send, and put an agent on what it adds"), line, put], chat_key); }
        return (vec![plain(enter.clone(), "add it as a task"), plain(tab, "add it and put an agent on it"), line, put], chat_key);
    }
    // The popup closes on its last esc: it says so, last on the line (not while esc would hide the chat's message).
    let closes = esc_step(s).is_empty() && s.ctx.popup && !crate::board::update::chat::shown(s);
    let back: Vec<Hint> = if !esc_step(s).is_empty() { vec![plain(esc.clone(), esc_step(s))] } else if closes { vec![Hint { key: esc.clone(), words: "close".into(), stays: true, ..Default::default() }] } else { vec![] };
    let leave: Option<Vec<Hint>> = if s.full { let mut v = vec![]; if s.panel_length > s.panel_height { v.push(Hint { key: arrows.clone(), words: "scroll the task".into(), stays: true, ..Default::default() }); } v.extend(back.clone()); Some(v) } else { None };
    if s.job().is_some() { let mut v = vec![hk(K::Stop, "stop sending this"), plain(enter, "take the words back to change")]; v.extend(leave.unwrap_or(vec![more])); return (v, chat_key); }
    if let Some(a) = s.auto() { return (auto_hints(s, a, &back, &more), chat_key); }
    if let Some(w) = crate::board::update::heads::says(s) { let mut v = vec![hk(K::Enter, &w)]; v.extend(back); v.extend(key); v.push(more); return (v, chat_key); }
    let Some(r) = r else { let mut v = back; v.extend(key); v.push(more); return (v, chat_key); };
    let tail: Vec<Hint> = leave.clone().unwrap_or_else(|| vec![if s.reply { Hint { stays: true, ..hk(K::Talk, "back to the Coordinator") } } else { more.clone() }]);
    let mid: Vec<Hint> = if leave.is_some() { vec![] } else { back };
    let failure = failure_for(r);
    let rv = crate::store::reviews::current_review(r).cloned();
    let sendable = rv.as_ref().map(|x| crate::gate::sendable(x)).unwrap_or(false);
    let open = h(hint(K::Enter, None));
    let fix = h(hint(K::SendBack, None));
    let stop = h(hint(K::Stop, None));
    let agent = h(hint(K::Delegate, None));
    let reply = if agent_on { h(hint(K::Talk, None)) } else { hk(K::Talk, "talk about this task") };
    let retry = if r.s("stage") == "failed" { h(hint(K::Yes, None)) } else { hk(K::Yes, "tell this task's agent to try again") };
    let as_is = hk(K::Yes, "retry as it is");
    let st = |x: &str| r.s("stage") == x;
    let at = if choosing(s, r) { attempt_now(s, r) } else { None };
    let opt = |b: bool, x: Hint| if b { Some(x) } else { None };
    let main: Vec<Option<Hint>> = if r.b("archivedAt") { vec![Some(h(hint(K::Archive, None))), Some(hk(K::Stop, "remove this task"))] }
        else if let (Some(c), true) = (&at, st("pick")) { let l = c.s("letter"); vec![Some(Hint { short: format!("keep {l}"), ..hk(K::Yes, &format!("keep attempt {l}")) }), Some(h(hint(K::NextAttempt, None))), Some(hk(K::Enter, &format!("open attempt {l}'s agent")))] }
        else if let Some(c) = &at { vec![Some(hk(K::Enter, &format!("open attempt {}'s agent", c.s("letter")))), Some(h(hint(K::NextAttempt, None))), Some(reply.clone()), Some(stop.clone())] }
        else if in_review(r) { vec![Some(Hint { short: "merge this task".into(), ..hk(K::Yes, "approve and merge this task") }), opt(sendable, hk(K::SendBack, "send the review to this task's agent")), Some(reply.clone())] }
        else if failed_review(r) { vec![opt(sendable, hk(K::SendBack, "send the review to this task's agent")), Some(Hint { short: "merge anyway".into(), ..hk(K::Yes, "merge this task anyway") }), Some(reply.clone())] }
        else if st("approval") && r.b("syncBlocked") { vec![Some(reply.clone()), Some(open.clone())] }
        else if st("approval") && r.s("state") == "blocked" { vec![opt(failure.as_ref().map(|f| f.b("agentCanFix") && !f.b("sentAt")).unwrap_or(false), fix.clone()), Some(hk(K::Yes, "retry the merge")), Some(reply.clone())] }
        else if st("stopped") && r.b("limitStop") { let rev = r.at("limitStop").s("role") == "reviewer"; vec![Some(hk(K::SendBack, if rev { "carry on with this task" } else { "tell this task's agent to continue" })), opt(!rev, hk(K::Yes, "use its work as it is")), Some(reply.clone())] }
        else if st("nosummary") { vec![Some(hk(K::SendBack, "tell this task's agent to continue")), Some(hk(K::Yes, "use its work as it is")), Some(open.clone())] }
        else if failure.as_ref().map(|f| f.b("agentCanFix")).unwrap_or(false) { if failure.as_ref().unwrap().b("sentAt") && r.s("state") == "working" { vec![Some(open.clone()), Some(reply.clone()), Some(stop.clone())] } else { vec![Some(fix.clone()), opt(st("failed"), as_is.clone()), Some(reply.clone())] } }
        else if failure.is_some() { vec![opt(st("failed") || r.s("kind") == "auth", retry.clone()), Some(reply.clone()), Some(open.clone())] }
        else if r.s("state") == "error" && can_send_error(r) { vec![Some(fix.clone()), opt(st("failed"), as_is.clone()), Some(reply.clone())] }
        else if st("plan") && r.s("state") == "blocked" { vec![Some(Hint { short: "approve the plan".into(), ..hk(K::Yes, "approve this task's plan") }), Some(hk(K::Talk, "send the plan back with your words")), Some(hk(K::Panel, "read the whole plan"))] }
        else if agent_on && r.s("state") == "blocked" && r.s("kind") == "permission" { vec![Some(hk(K::Yes, "say yes to this task's agent")), Some(reply.clone()), Some(open.clone())] }
        else if agent_on && r.s("state") == "blocked" { vec![Some(if r.s("kind") == "question" { hk(K::Talk, "answer this task's agent") } else { reply.clone() }), Some(open.clone())] }
        else if stuck_finishing(r) { vec![Some(hk(K::Yes, "finish with what its agent said last")), Some(open.clone()), Some(reply.clone())] }
        else if agent_on && r.s("state") == "working" { vec![Some(open.clone()), Some(reply.clone()), Some(stop.clone())] }
        else if agent_on { vec![Some(open.clone()), Some(reply.clone())] }
        else if r.s("state") == "working" { vec![Some(agent.clone()), Some(hk(K::Stop, "put it back in to do"))] }
        else { vec![Some(agent.clone()), Some(reply.clone())] };
    // While you talk to this task's agent, space is in the tail: it goes back to the Coordinator.
    let talk = name(K::Talk);
    let mut main: Vec<Hint> = main.into_iter().flatten().filter(|x| !(s.reply && !r.b("archivedAt") && x.key == talk)).collect();
    // The first hint that names it says whose agent, "this task's"; after that it is "the agent".
    let mut named = false;
    for x in main.iter_mut() { if named { x.words = x.words.replace("this task's agent", "the agent"); } named = named || x.words.contains("this task"); }
    let room = (4 + usize::from(closes)).saturating_sub(tail.len() + mid.len());
    main.truncate(room);
    let mut out = main; out.extend(mid); out.extend(key); out.extend(tail);
    (out, chat_key)
}
/// The hint line on an automation's row: only keys that act there, the main thing first.
fn auto_hints(s: &State, a: &Obj, back: &[Hint], more: &Hint) -> Vec<Hint> {
    let mut act = vec![hk(K::Talk, &format!("turn this automation {}", if a.b("on") { "off" } else { "on" }))];
    if a.b("on") { act.push(hk(K::Delegate, "fire it now")); }
    act.push(hk(K::Stop, "remove it"));
    if s.ctx.web { let mut v = act; if !s.auto_open { v.extend(back.iter().cloned()); } return v; }
    if !s.auto_open {
        let mut v = vec![plain(name(K::Enter), "open this automation")];
        v.extend(act);
        if crate::board::update::autos::can_set(a) { v.push(hk(K::Settings, "its settings")); }
        if back.is_empty() { v.push(more.clone()); } else { v.extend(back.iter().cloned()); }
        return v;
    }
    let all = crate::board::update::autos::firings(a);
    let f = usize::try_from(s.auto_at).ok().and_then(|n| all.get(n));
    let mut v = vec![];
    if !all.is_empty() { v.push(plain([name(K::Up), name(K::Down)].join(""), "pick a firing")); }
    if f.map(|f| f.b("task")).unwrap_or(false) { v.push(plain(name(K::Enter), "go to its task")); }
    v.extend(act.into_iter().take(2));
    v.push(Hint { key: name(K::Back), words: "back to the list".into(), stays: true, ..Default::default() });
    v
}
