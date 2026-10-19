//! The attempts are through: the check filtered, now the picker chooses. None passed: the task failed, with every
//! attempt kept to look at. One passed: it is the winner and nobody is asked. Several: `attempts.pick`, read now, says
//! who chooses. `you`: the task waits as "pick one", with the reviewer's comparison when there is a reviewer.
//! `reviewer`: the reviewer compares them and names one, and that one goes on by itself.
use super::adopt::adopt;
use super::words::{agent_word, out_why, pick_line, standing_of};
use crate::actions::shared::{say, Said};
use crate::board::keys::key_name;
use crate::clock::now;
use crate::err::Res;
use crate::json::*;
use crate::runner::{runner_alive, spawn_runner};
use crate::store::attempts::attempts_of;
use crate::store::failures::failure_patch;
use crate::store::settings::lookup::setting_text;
use crate::store::tasks::{assert_unarchived, find, get, update_wait, Change, Task};

const LOST_MS: i64 = 60000;
/// Called by the housekeeping pass for every task whose attempts are under way.
pub fn settle_pass(tasks: &[Obj], socket: &str, now: i64) {
    for t in tasks {
        if t.b("archivedAt") || t.at("link").s("socket") != socket || runner_alive(t) { continue; }
        if t.s("stage") == "attempts" { let _ = settle(t.s("id"), now); }
        // A comparison whose runner is gone (the machine restarted): nobody will name one, so the task waits for your pick.
        else if t.s("stage") == "reviewing" && t.b("comparing") && now - t.at("comparing").i("at") > LOST_MS { let _ = to_pick(t.s("id"), Some(json!({ "error": "the comparison stopped before the reviewer named an attempt" })), "reviewing"); }
    }
}
pub fn settle(id: &str, now: i64) -> Res<Option<Task>> {
    let Some(t) = get(id) else { return Ok(None) };
    if t.s("stage") != "attempts" { return Ok(Some(t)); }
    let kids = attempts_of(&t);
    let st: Vec<&str> = kids.iter().map(|c| standing_of(c, now)).collect();
    if kids.is_empty() || st.iter().any(|s| ["works", "waits", "needs"].contains(s)) || kids.iter().any(runner_alive) { return Ok(Some(t)); }
    let passed: Vec<&Task> = kids.iter().zip(&st).filter(|(_, s)| **s == "passed").map(|(c, _)| c).collect();
    if passed.is_empty() {
        let lines: Vec<String> = kids.iter().map(|c| format!("{} {}: {}{}", c.s("letter"), agent_word(c), out_why(c), if c.at("failure").b("title") { format!(" ({})", c.at("failure").s("title")) } else { String::new() })).collect();
        return Ok(Some(update_wait(id, 10000, |t| {
            if t.s("stage") != "attempts" { return Ok(Change::Keep); }
            t.put("stage", "failed"); t.put("msg", format!("No attempt passed: {}", lines.join(" · ")));
            for (k, v) in failure_patch(&obj(json!({ "stage": "attempts", "auth": false, "agentCanFix": false, "title": "No attempt passed",
                "detail": format!("All {} attempts are out:\n{}\n\nEach one's worktree, branch and pane are kept, to look at.", kids.len(), lines.iter().map(|l| format!("- {l}")).collect::<Vec<_>>().join("\n")),
                "next": format!("Look at what each attempt did in its pane. Delegate the task again to start over with fresh attempts ({}); the old ones are removed then.", key_name("delegate")) }))) { t.insert(k, v); }
            Ok(Change::Write)
        })?));
    }
    if passed.len() == 1 { return adopt(id, passed[0].s("letter"), "check").map(Some); }
    // Read now, as this step starts: who picks, and whether there is a reviewer to compare them.
    let pick = setting_text("attempts.pick", Some(&t));
    let reviewer = setting_text("review.when", Some(&t)) != "never";
    if reviewer {
        let c = update_wait(id, 10000, |t| { if t.s("stage") != "attempts" { return Ok(Change::Keep); } t.put("stage", "reviewing"); t.put("comparing", json!({ "pick": pick, "at": now })); t.put("reviewAt", now); t.put("msg", "Comparing the attempts"); t.put("watch", Value::Null); t.put("comparison", Value::Null); Ok(Change::Write) })?;
        if c.s("stage") == "reviewing" && c.b("comparing") { spawn_runner("compare", id, &[]); }
        return Ok(Some(c));
    }
    to_pick(id, if pick == "reviewer" { Some(json!({ "error": "attempts.pick is reviewer, but review.when is never: there is no reviewer to choose" })) } else { None }, "attempts").map(Some)
}
/// The task waits for you to choose. `comparison`: what the reviewer said of the attempts, or why it said nothing.
pub fn to_pick(id: &str, comparison: Option<Value>, from: &str) -> Res<Task> {
    update_wait(id, 10000, |t| {
        if t.s("stage") != from { return Ok(Change::Keep); }
        if let Some(c) = &comparison { t.put("comparison", c.clone()); }
        t.put("stage", "pick"); t.put("comparing", Value::Null); t.put("watch", Value::Null); t.put("kind", "permission");
        let line = pick_line(t, None); t.put("msg", line);
        Ok(Change::Write)
    })
}
/// Your choice (the approve key on an attempt, or `herdr-projects pick <id> <letter>`). With no letter: the one the
/// reviewer recommends, when it recommended one.
pub fn pick(id: &str, letter: &str) -> Res<Said> {
    let t = find(id)?;
    assert_unarchived(&t)?;
    let tid = t.t("id");
    if t.s("stage") != "pick" {
        if t.arr("attempts").is_empty() { crate::fail!("{tid} was not done in several attempts."); }
        crate::fail!("{tid} does not wait for a pick{}.", if t.b("picked") { format!(": attempt {} was chosen", t.at("picked").s("letter")) } else if t.s("stage") == "attempts" { ": its attempts are still under way".into() } else { String::new() });
    }
    let n = now();
    let kids = attempts_of(&t);
    let passed: Vec<&Task> = kids.iter().filter(|c| standing_of(c, n) == "passed").collect();
    let want = if letter.is_empty() { t.at("comparison").t("pick") } else { letter.to_string() }.to_uppercase();
    let names = passed.iter().map(|c| c.t("letter")).collect::<Vec<_>>();
    if want.is_empty() { crate::fail!("Say which attempt: herdr-projects pick {tid} <{}>.", names.join("|")); }
    let Some(kid) = passed.iter().find(|c| c.s("letter") == want) else {
        if kids.iter().any(|c| c.s("letter") == want) { crate::fail!("Attempt {want} did not pass the check, so it cannot be chosen. Those that passed: {}.", names.join(", ")); }
        crate::fail!("{tid} has no attempt {want}. Those that passed: {}.", names.join(", "));
    };
    let word = agent_word(kid);
    let task = adopt(&tid, &want, "you")?;
    Ok(say(task, format!("{tid}: attempt {want} ({word}) chosen. It goes on to review; the other attempts' panes close.")))
}
