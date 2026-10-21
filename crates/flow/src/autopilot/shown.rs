//! A coordinator's autopilot as it is shown, in the same words on its row, in its panel, on the command line and to
//! another machine: nothing while it is off. On, its state word (shown first wins: `needs you`, `could not start`,
//! `working`, `asleep`, `at its limit`, `on`), its line laid out as an automation's row
//! (`autopilot on · every 1h + events · last HSW-92 · next in 40m`), its goal, today's counts, its settings in plain
//! words (`*` on this time's) and its last wakes.
use super::care::needing;
use super::limits::{board_tasks, room, today};
use super::record::{get, scope_of, tasks, words};
use crate::coordinator::co::Co;
use crate::json::*;
use crate::shown::words::age;
use crate::store::keys::{repo_keys, say_id};
use crate::store::settings::lookup::lookup;
use crate::store::settings::pilot_values::awake;
use crate::store::settings::places::yours;
use crate::store::settings::values::ms;

/// Its state word now; "" while it is off.
pub fn state_word(co: &Co, ap: &Obj, board: &[Obj], now: i64) -> &'static str {
    if !ap.b("on") { return ""; }
    if ap.b("needsYou") || words(co, "coordinator.access") == "ask" { return "needs you"; }
    if !ap.s("couldNot").is_empty() { return "could not start"; }
    if ap.at("wake").b("typedAt") { return "working"; }
    if !awake(&words(co, "autopilot.hours"), now) { return "asleep"; }
    if !room(co, ap, board, now) && needing(ap, board, words(co, "autopilot.approve") == "yes").is_empty() { return "at its limit"; }
    "on"
}
/// Its triggers in plain words: "every 1h + events", "events", "every 6h", "no timer, no events".
pub fn triggers(co: &Co) -> String {
    let (every, when) = (words(co, "autopilot.every"), words(co, "autopilot.when"));
    match (every != "none", when != "none") { (true, true) => format!("every {every} + events"), (true, false) => format!("every {every}"), (false, true) => "events".into(), _ => "no timer, no events".into() }
}
/// When it next wakes, in words: "next in 40m", "waits for its hours", "" (nothing to say).
pub fn next_words(co: &Co, ap: &Obj, now: i64) -> String {
    if !awake(&words(co, "autopilot.hours"), now) { return "waits for its hours".into(); }
    let ended = ap.i("endAt").max(ap.i("since"));
    let gap = ended + ms(&words(co, "autopilot.gap"));
    if !ap.arr("pending").is_empty() { return if gap > now { format!("next in {}", age(gap - now)) } else { String::new() }; }
    let every = ms(&words(co, "autopilot.every"));
    if every == 0 { return String::new(); }
    let at = (ended.max(ap.i("timerAt")) + every).max(gap);
    if at > now { format!("next in {}", age(at - now)) } else { String::new() }
}
/// The last task it added, as the board names it ("" none).
pub fn last_added(ap: &Obj) -> String { tasks(ap).into_iter().filter(|t| t.s("by") == "added").max_by_key(|t| t.i("at")).map(|t| say_id(t.s("id"), &repo_keys())).unwrap_or_default() }
/// The line under its heading: `autopilot on · every 1h + events · last HSW-92 · next in 40m`.
pub fn line_of(co: &Co, ap: &Obj, board: &[Obj], now: i64) -> String {
    let st = state_word(co, ap, board, now);
    if st.is_empty() { return String::new(); }
    let state = if st == "could not start" { format!("could not start: {}", ap.s("couldNot")) } else { st.to_string() };
    let last = last_added(ap);
    [format!("autopilot {state}"), triggers(co), if last.is_empty() { String::new() } else { format!("last {last}") }, next_words(co, ap, now)].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ")
}
/// Its settings as the panel says them, each with whether this time's value it is.
pub fn settings_of(co: &Co) -> Vec<(String, String, bool)> {
    let (scope, mine) = (scope_of(co), yours());
    let get = |n: &str| lookup(n, &scope, &mine).map(|f| (text(&f.value), f.place == "once")).unwrap_or_default();
    let w = |n: &str| get(n).0;
    let once = |ns: &[&str]| ns.iter().any(|n| get(n).1);
    let every = w("autopilot.every");
    let when = w("autopilot.when");
    vec![
        ("Wakes".into(), format!("{} · {}", if every == "none" { "no timer".into() } else { format!("every {every}") }, if when == "none" { "no events".into() } else { format!("on {when}") }), once(&["autopilot.every", "autopilot.when"])),
        ("Hours".into(), w("autopilot.hours"), once(&["autopilot.hours"])),
        ("Limits".into(), format!("{} tasks a day · {} at once · {} wakes a day · gap {}", w("autopilot.tasksPerDay"), w("autopilot.tasksAtOnce"), w("autopilot.wakesPerDay"), w("autopilot.gap")), once(&["autopilot.tasksPerDay", "autopilot.tasksAtOnce", "autopilot.wakesPerDay", "autopilot.gap"])),
        ("May set".into(), if w("autopilot.mayChange") == "none" { "nothing".into() } else { w("autopilot.mayChange") }, once(&["autopilot.mayChange"])),
        ("May touch".into(), if w("autopilot.scope") == "all" { "its own and your to-do tasks".into() } else { "only its own tasks".into() }, once(&["autopilot.scope"])),
        ("Says yes".into(), if w("autopilot.approve") == "yes" { "for the tasks it looks after".into() } else { "never (merge.by decides)".into() }, once(&["autopilot.approve"])),
        ("Also".into(), format!("retries {} · {} · starts tasks {} · needs you after {} · settings file {}", w("autopilot.retries"), if w("autopilot.releases") == "yes" { "may add releases" } else { "no releases" }, if w("autopilot.startAtOnce") == "yes" { "at once" } else { "when it says" }, w("autopilot.needsYouAfter"), if w("autopilot.repoFileWaits") == "no" { "may merge" } else { "waits for you" }),
            once(&["autopilot.retries", "autopilot.releases", "autopilot.startAtOnce", "autopilot.needsYouAfter", "autopilot.repoFileWaits", "autopilot.switchOthers"])),
    ]
}
/// Everything shown of it, as one record: what this board draws, and what another machine is sent. Off: `{ on: false }`.
pub fn view(co: &Co, now: i64) -> Value {
    let ap = get(co);
    if !ap.b("on") { return json!({ "on": false }); }
    let board = board_tasks();
    let wakes: Vec<Value> = ap.arr("wakes").iter().rev().cloned().collect();
    json!({ "on": true, "state": state_word(co, &ap, &board, now), "line": line_of(co, &ap, &board, now), "goal": words(co, "autopilot.goal"),
        "today": today(co, &ap, &board, now).say(), "couldNot": ap.at("couldNot"), "next": next_words(co, &ap, now), "last": last_added(&ap),
        "settings": settings_of(co).into_iter().map(|(l, v, o)| json!({ "label": l, "value": v, "once": o })).collect::<Vec<_>>(), "wakes": wakes })
}
/// What the main Coordinator's row sums up of the others: "2 others on", "1 needs you" ("" none on).
pub fn others_words(views: &[Value]) -> String {
    let on = views.iter().filter(|v| v.b("on")).count();
    if on == 0 { return String::new(); }
    let need = views.iter().filter(|v| v.b("on") && ["needs you", "could not start"].contains(&v.s("state"))).count();
    let mut out = vec![format!("{on} {} on", if on == 1 { "other" } else { "others" })];
    if need > 0 { out.push(format!("{need} {} you", if need == 1 { "needs" } else { "need" })); }
    out.join(" · ")
}
