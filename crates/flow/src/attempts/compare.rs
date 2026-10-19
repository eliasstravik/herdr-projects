//! The reviewer's comparison of the attempts that passed the check: it reads each one's change, says what is good and
//! bad about each, names one and says why. A visible herdr agent in a tiled pane (`Picker`) in the task's space. With
//! `attempts.pick reviewer` the one it names goes on by itself; with `you` its comparison stands on top of the attempts
//! you choose from. For a research-only task it reads the reports instead, and, when it picks, writes one report from all.
use super::adopt::adopt;
use super::settle::to_pick;
use super::words::{agent_word, standing_of};
use crate::agents::{helpers::still_at_work, installed, start};
use crate::board_rule::reviewer_board_rule;
use crate::clock::now;
use crate::delegate::{launch_extras, rules::task_line_as};
use crate::err::Res;
use crate::flow;
use crate::json::*;
use crate::research::{is_research, report_folder};
use crate::runner::{claim, is_aborted, Guard};
use crate::store::attempts::attempts_of;
use crate::store::files::{board_env, dir_of};
use crate::store::marks::pending_events;
use crate::store::repos::folder_of;
use crate::store::settings::lookup::work;
use crate::store::tasks::{get, update_wait, Change, Task};
use std::path::{Path, PathBuf};

fn ms(name: &str, d: i64) -> i64 { crate::finish::shared::ms(name, d) }
pub fn compare_dir(t: &Obj) -> PathBuf { dir_of(t.s("id")).join("compare") }
/// What the reviewer is told. `writes`: it also writes one report from all of them.
pub fn compare_brief(t: &Task, passed: &[Task], writes: bool) -> String {
    let dir = compare_dir(t);
    let research = is_research(t);
    let one = |c: &Task| {
        let base = [c.s("syncedTo"), c.s("baseSha"), c.s("from")].into_iter().find(|s| !s.is_empty()).unwrap_or("HEAD").to_string();
        let said: String = crate::text::squash(if c.b("summary") { c.s("summary") } else { c.s("said") }).chars().take(1200).collect();
        [format!("Attempt {}, by {}:", c.s("letter"), agent_word(c)),
            if research { format!("  its report: {}/", report_folder(c).display()) } else { format!("  worktree {}, branch {}{}", c.s("worktree"), c.s("branch"), if c.b("stat") { format!(" ({})", c.s("stat")) } else { String::new() }) },
            if research { String::new() } else { format!("  its change: git -C {} diff {base}...HEAD", c.s("worktree")) },
            format!("  what its agent says it did: {}", if said.is_empty() { "nothing".into() } else { said })].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n")
    };
    let desc: String = t.s("description").chars().take(3000).collect();
    let letters = passed.iter().map(|c| c.t("letter")).collect::<Vec<_>>().join(" or ");
    [format!("You compare {} attempts at one task. Each was made by an agent of its own that did not see the others{}. One of them will be kept and the others thrown away: nothing of two attempts is ever put together{}. Your job is to say which one, and why.", passed.len(), if research { "" } else { ", in a worktree and on a branch of its own, and each passed the repo's check" }, if research { " in the repo" } else { "" }),
        if research { "This task is research only: each attempt's whole result is a report. Read every report whole, and check what they claim where you can.".into() } else { "Read each attempt's whole change and what surrounds it, and try each one: run it, look at what it does with the awkward inputs. Judge them against what was asked: which does all of it, correctly, with the least risk? A smaller change that does the job beats a larger one. Ignore taste and style.".to_string() },
        "You are here to compare, not to fix. Change, commit, push and merge nothing, in any of the worktrees.".into(),
        reviewer_board_rule(),
        format!("Task: {}", t.s("title")), if desc.is_empty() { String::new() } else { format!("Details:\n{desc}") },
        passed.iter().map(one).collect::<Vec<_>>().join("\n\n"),
        if writes { format!("Then write one report from all of them: the best of each, what they agree on, and where they differ and which is right. Save it in {}/ as index.html (one page that stands by itself) or report.md. It is the task's report; the attempts' own reports are kept beside it only until the task is done.", report_folder(t).display()) } else { String::new() },
        format!("Last of all, save your answer as JSON in {}/comparison.json, then stop. The board waits for that file. Its shape: {{\"pick\": \"<the letter of the attempt to keep: {letters}>\", \"why\": \"<two or three plain sentences: why that one>\", \"attempts\": [{{\"letter\": \"<its letter>\", \"note\": \"<one or two sentences: what is good and what is bad about it>\"}}, ...]}}", dir.display()),
    ].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n")
}
/// The answer as the board keeps it, or an error that says what is wrong with it.
pub fn read_comparison(raw: &str, letters: &[String]) -> Res<Obj> {
    let j: Value = serde_json::from_str(raw)?;
    let want = j.t("pick").trim().to_uppercase();
    if !letters.contains(&want) { crate::fail!("it named \"{}\", which is none of {}", j.t("pick").chars().take(20).collect::<String>(), letters.join(", ")); }
    let line = |s: &Value, n: usize| crate::text::squash(&text(s)).chars().take(n).collect::<String>();
    let notes: Obj = j.arr("attempts").iter().map(|a| (a.t("letter").to_uppercase(), json!(line(a.at("note"), 600)))).filter(|(l, n)| letters.contains(l) && n != &json!("")).collect();
    Ok(obj(json!({ "pick": want, "why": line(j.at("why"), 1200), "notes": notes, "at": now() })))
}
fn close_picker(id: &str) { crate::finish::review::close_reviewer(id); }
pub fn compare_job(id: &str) {
    let Ok(Some(t)) = claim(id, "compare", |_| {}) else { return };
    if t.s("stage") != "reviewing" || !t.b("comparing") { crate::runner::release(id, false); return; }
    let mut g = Guard::new(id);
    g.expect(&t, &[]);
    let out = match compare(id, &mut g, &t) { Ok(o) => Some(o), Err(e) if is_aborted(&e) => None, Err(e) => Some(obj(json!({ "error": format!("the comparison failed: {}", crate::text::squash(&e.msg).chars().take(300).collect::<String>()) }))) };
    close_picker(id);
    // The runner lets go first: what follows (the chosen attempt's finish line) is a runner of its own.
    let _ = update_wait(id, 10000, |t| { if t.at("runner").i("pid") != std::process::id() as i64 { return Ok(Change::Keep); } t.put("runner", Value::Null); let s = t.i("eventSeq"); t.put("eventsHandled", s); t.put("events", json!([])); Ok(Change::Write) });
    let Some(mut comparison) = out else { return };
    let Some(cur) = get(id) else { return };
    if cur.s("stage") != "reviewing" || !cur.b("comparing") { return; }
    if cur.at("comparing").s("pick") == "reviewer" && comparison.b("pick") {
        let c2 = comparison.clone();
        let r = update_wait(id, 10000, |t| { t.put("comparison", c2.clone()); t.put("comparing", Value::Null); Ok(Change::Write) }).and_then(|_| adopt(id, comparison.s("pick"), "reviewer"));
        match r { Ok(_) => return, Err(e) => comparison.put("error", e.msg) }
    }
    let _ = to_pick(id, Some(Value::Object(comparison)), "reviewing");
}
fn compare(id: &str, g: &mut Guard, t: &Task) -> Res<Obj> {
    let api = flow::task::h(t);
    let dir = compare_dir(t);
    let file = dir.join("comparison.json");
    let n = now();
    let passed: Vec<Task> = attempts_of(t).into_iter().filter(|c| standing_of(c, n) == "passed").collect();
    let letters: Vec<String> = passed.iter().map(|c| c.t("letter")).collect();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let ch = work(Some(t));
    let who = start::role_of("reviewer", &|n| ch.at(n).clone());
    let (mut k, mut model, mut effort) = (who.kind.clone(), who.model.clone(), who.effort.clone());
    if !installed::installed(&k) && passed.first().map(|p| installed::installed(p.s("agent"))).unwrap_or(false) { k = passed[0].t("agent"); model = String::new(); effort = String::new(); }
    let writes = is_research(t) && t.at("comparing").s("pick") == "reviewer";
    let brief = format!("{}\n\n{}{}", task_line_as(t, "reviewer", 0), compare_brief(t, &passed, writes), if who.instructions.is_empty() { String::new() } else { format!("\n\n{}", start::own_words(&who.instructions)) });
    let cwd = passed.first().map(|c| c.t("worktree")).filter(|w| !w.is_empty() && Path::new(w).exists()).map(PathBuf::from).unwrap_or_else(|| folder_of(t.s("id")));
    installed::pre_trust(&k, &folder_of(t.s("id")), &who.access);
    close_picker(id);
    let ws = t.at("link").t("workspace");
    let err = |e: &str| Ok(obj(json!({ "error": e, "by": k })));
    if ws.is_empty() || api.workspace_get(&ws).is_none() { return Ok(obj(json!({ "error": "the task's space is gone, so nobody compared the attempts" }))); }
    let anchor = if t.at("link").b("attemptAnchorPane") { t.at("link").t("attemptAnchorPane") } else { t.at("link").t("pane") };
    let made = api.pane_split(&anchor, &cwd.display().to_string(), "down", &board_env())?;
    let tab = made.t("tab_id");
    let pane = made.t("pane_id");
    if pane.is_empty() { return Ok(obj(json!({ "error": "herdr opened no terminal pane for the comparison" }))); }
    let taken: Vec<String> = api.agents().unwrap_or_default().iter().map(|a| a.t("name")).collect();
    let repo: String = t.s("repo").to_lowercase().chars().map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() { c } else { '-' }).take(16).collect();
    let mut name: String = format!("sw-{repo}-{}-pick", t.t("n")).chars().take(32).collect();
    if taken.contains(&name) { name = format!("{}-{:04}", name.chars().take(27).collect::<String>(), now() % 10000); }
    let started_at = now();
    let x = launch_extras(t, "reviewer", &k, 0, "", None)?;
    g.set(|t| { t.obj_mut("link").put("reviewer", json!({ "pane": pane, "tab": tab, "agent": name, "kind": k })); t.put("reviewSession", x.session.clone().map(Value::String).unwrap_or(Value::Null)); })?;
    let start_with = |hook: Vec<String>| -> Res<()> {
        let args = start::start_args(&k, &start::StartOpts { model: model.clone(), effort: effort.clone(), fast: Some(who.fast), access: who.access.clone(), session: x.session.clone().unwrap_or_default(), extra: hook, ..Default::default() })?;
        match api.agent_start(&name, &k, &pane, &args, 60000) { Ok(_) => { crate::submit::settle(&api, &pane, &std::iter::once(crate::agents::kinds::program(&k).to_string()).chain(args).collect::<Vec<_>>().join(" ")); Ok(()) } Err(e) if e.code == "agent_not_ready" => Ok(()),
            Err(e) => Err(crate::agents::start_failed::say(&api, &crate::agents::start_failed::Start { kind: &k, setting: "reviewer.agent", again: "pick again" }, &pane, &e.message, &crate::store::files::dir_of(t.s("id")).join("runner.log")).into()) }
    };
    if let Err(e) = start_with(x.hook.clone()) { if x.hook.is_empty() { return Err(e); } start_with(vec![])?; }
    let until = crate::finish::shared::review_until(&ch);
    let every = ms("HERDR_PROJECTS_POLL_MS", 3000);
    let (mut sent, mut comp_at_send, mut quiet_since) = (false, Value::Null, 0i64);
    let reads = crate::agents::kinds::kind(&k).map(|a| a.reads_turn).unwrap_or(false);
    let mut waited: Vec<i64> = vec![]; // its own turn-end reports while helpers it started were still at work
    let sleep = |m: i64| std::thread::sleep(std::time::Duration::from_millis(m.max(0) as u64));
    loop {
        g.check()?;
        if file.exists() {
            sleep(every.min(300)); // let the write complete
            return match std::fs::read_to_string(&file).map_err(crate::err::Error::from).and_then(|r| read_comparison(&r, &letters)) {
                Ok(mut c) => { c.put("by", k.clone()); if writes { c.put("wrote", true); } Ok(c) }
                Err(e) => err(&format!("the reviewer's comparison could not be read: {}", crate::text::squash(&e.msg).chars().take(300).collect::<String>())),
            };
        }
        if until.is_some_and(|u| now() > u) { return err(&crate::finish::shared::timed_out("the comparison", &ch)); }
        // The reviewer's own turn-end report, and still no answer: it finished without one.
        let ended = get(id).and_then(|c| pending_events(&c).into_iter().find(|e| e.s("role") == "reviewer" && !e.b("side") && e.i("at") >= started_at && !waited.contains(&e.i("n"))));
        if let (Some(e), true) = (&ended, sent) {
            sleep(every.min(1500));
            if file.exists() { continue; }
            let w = crate::turn::why_stopped(&get(id).unwrap_or(t.clone()), "reviewer", Some(e));
            if w.turn.as_ref().map(|x| x.error.is_empty()).unwrap_or(false) && still_at_work(&w.kind, w.turn.as_ref(), started_at) { waited.push(e.i("n")); continue; }
            return err("the reviewer finished without naming an attempt");
        }
        let a = api.agent_get(&pane).ok().flatten();
        if a.is_none() && sent { return err("the reviewer exited without naming an attempt"); }
        if a.is_none() && api.pane_get(&pane).is_none() { return err("the reviewer's terminal was closed"); }
        let at_rest = a.as_ref().map(|a| a.s("agent_status") == "idle" || a.s("agent_status") == "done").unwrap_or(false);
        if !sent && at_rest {
            comp_at_send = a.as_ref().map(|a| a.at("completion_seq").clone()).unwrap_or(Value::Null);
            match crate::submit::hand(&api, &pane, &brief) { Ok(_) => sent = true, Err(e) => if e.code != "agent_blocked" && e.code != "agent_not_ready" { sent = true; } }
        } else if sent && !reads {
            // A program whose conversation the board does not read: herdr saw it finish a turn since the brief, and
            // still no answer 15 seconds later.
            let done = at_rest && a.as_ref().map(|a| !a.at("completion_seq").is_null() && *a.at("completion_seq") != comp_at_send).unwrap_or(false);
            if !done { quiet_since = 0; } else if quiet_since == 0 { quiet_since = now(); } else if now() - quiet_since > ms("HERDR_PROJECTS_QUIET_MS", 15000) && !file.exists() { return err("the reviewer finished without naming an attempt"); }
        }
        sleep(every);
    }
}
