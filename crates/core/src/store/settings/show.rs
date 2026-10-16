//! `show`: every setting, its value and the place it came from, for you or for one task; and what is wrong in a file
//! somebody edited by hand.
use super::lookup::{in_repo, lookup_at, places_of, Found, Scope};
use super::places::{repo_file, yours};
use super::values::{agent_of, say};
use crate::agents::kinds::kind;
use crate::json::*;
use crate::research::later_words;
use crate::store::keys::repo_keys;
use crate::store::old_research::RESEARCH;
use crate::store::repos::folder_of;
use crate::store::settings_list::{needs_repo, Entry, Kind, Ty, SETTINGS};
use crate::store::tasks::Task;

pub struct Row { pub name: String, pub kind: Kind, pub value: Value, pub from: String, pub place: String, pub own: bool, pub note: String, pub later: String }
impl Row {
    /// As the command line's --json and another machine read it.
    pub fn json(&self) -> Value { let mut o = json!({ "name": self.name, "kind": self.kind.name(), "value": self.value, "from": self.from, "place": self.place, "own": self.own, "note": self.note }); if !self.later.is_empty() { o.as_object_mut().unwrap().put("later", self.later.as_str()); } o }
}

/// Why a value does nothing where it stands: the first reason that says something is the note.
fn note(s: &Entry, r: &Found, task: Option<&Task>, scope: &Scope, no_repo: bool, get: &dyn Fn(&str) -> String) -> String {
    if no_repo && needs_repo(&s.name) { return "does not apply: this task is in no repo".into(); }
    if s.name == "repo.key" && text(&r.value) != "none" && (task.is_some() || !scope.repo.is_empty()) {
        let repo = if scope.repo.is_empty() { task.map(|t| t.s("id").split('/').next().unwrap_or("").to_string()).unwrap_or_default() } else { scope.repo.clone() };
        match repo_keys().get(&repo) { Some(k) if *k == text(&r.value) => {}, Some(k) => return format!("not used: another repo has or had the same key; its tasks go by {k}"), None => return "not used: another repo has or had the same key".into() }
    }
    if s.name == "attempts.by" && task.map(|t| t.b("startedOnce")).unwrap_or(false) { return "its work has begun: counts when it is started afresh".into(); }
    if s.name.ends_with(".fast") && r.value == json!(true) {
        let k = agent_of(&get(&format!("{}.agent", s.name.split('.').next().unwrap_or("")))).kind;
        if !k.is_empty() && !kind(&k).map(|a| a.fast).unwrap_or(false) { return format!("does nothing: {k} has no fast mode"); }
    }
    // A program whose tokens cannot be counted adds nothing to the count.
    if s.name == "limit.tokens" && text(&r.value) != "none" {
        let mut kinds: Vec<String> = vec![];
        let worker = task.map(|t| t.t("agent")).filter(|a| !a.is_empty()).unwrap_or_else(|| agent_of(&get("worker.agent")).kind);
        let reviewer = if get("review.when") == "never" { String::new() } else { agent_of(&get("reviewer.agent")).kind };
        for k in [worker, reviewer] { if !k.is_empty() && !kind(&k).map(|a| a.tokens).unwrap_or(false) && !kinds.contains(&k) { kinds.push(k); } }
        if !kinds.is_empty() { return format!("does nothing for {}: its tokens cannot be counted", kinds.join(" and ")); }
    }
    // A list of agents does nothing where its only use is to hand over to the one that is on it anyway.
    if s.ty == Ty::Agents {
        if let Some(list) = r.value.as_array() {
            let role = s.name.split('.').next().unwrap_or("");
            let on = get(&format!("{role}.agent"));
            if !list.is_empty() && list.iter().all(|a| text(a) == on) { return format!("does nothing: {} is the {} already", text(&list[0]), if role == "worker" { "agent" } else { "reviewer" }); }
        }
    }
    String::new()
}
/// One row per setting.
pub fn rows(scope: &Scope) -> Vec<Row> {
    let mine = yours();
    let task = scope.task;
    let at = places_of(scope, &mine);
    let get = |name: &str| lookup_at(name, scope, &at).map(|f| text(&f.value)).unwrap_or_default();
    SETTINGS.iter().filter(|s| task.is_some() || if !scope.repo.is_empty() { s.kind != Kind::Board } else { s.kind != Kind::Repo }).filter_map(|s| {
        let r = lookup_at(&s.name, scope, &at).ok()?;
        let n = note(s, &r, task, scope, at.no_repo, &get);
        let own_place = if task.is_some() { "task" } else if !scope.repo.is_empty() { "repo" } else { "you" };
        let later = match task { Some(t) if s.name == RESEARCH => later_words(t, &r.value), _ => String::new() };
        Some(Row { name: s.name.clone(), kind: s.kind, own: r.place == own_place, value: r.value.clone(), from: r.from.clone(), place: r.place.clone(), note: n, later })
    }).collect()
}
/// What is wrong in the files, one sentence each. The board keeps running: each such setting falls to the next place.
pub fn problems(task: Option<&Task>) -> Vec<String> {
    let scope = task.map(Scope::task).unwrap_or_default();
    let mut out = problems_in(&scope);
    if let Some(t) = task {
        let refused: Vec<String> = if t.at("repoRefused").is_array() { strings(t.at("repoRefused")) }
            else if !t.b("repoFile") && in_repo(t) { repo_file(&folder_of(t.s("id")), None).1 } else { vec![] };
        out.extend(refused);
    }
    out
}
/// The same for any scope (yours, a repo's, a task's), without what only a task's own record holds.
pub fn problems_in(scope: &Scope) -> Vec<String> {
    let mine = yours();
    let at = places_of(scope, &mine);
    let mut out: Vec<String> = mine.problems.iter().chain(at.problems.iter()).map(|p| format!("{}: {}", p.place, p.why)).collect();
    for s in SETTINGS.iter().filter(|s| (s.ty == Ty::Effort || s.ty == Ty::Access) && (scope.task.is_none() && scope.repo.is_empty() || s.kind == Kind::Work)) {
        if let Ok(f) = lookup_at(&s.name, scope, &at) { out.extend(f.skipped); }
    }
    out
}
pub fn show_text(scope: &Scope) -> String {
    let list = rows(scope);
    let task = scope.task;
    let wide = list.iter().map(|r| r.name.len()).max().unwrap_or(0);
    let val = list.iter().map(|r| say(&r.value).split('\n').next().unwrap_or("").chars().count()).filter(|n| *n <= 40).max().unwrap_or(0);
    let head = match task { Some(t) => format!("{}: every setting, and the place it came from (* = this task's own)", t.s("id")),
        None if !scope.repo.is_empty() => format!("Your entry for {} (* = said there)", scope.repo), None => "Your settings (* = changed by you)".into() };
    let mut lines: Vec<String> = list.iter().map(|r| {
        let v: String = say(&r.value).split('\n').map(|l| l.trim()).collect::<Vec<_>>().join(" / ");
        format!("{} {:<wide$}  {:<val$}  {}{}", if r.own { "*" } else { " " }, r.name, v, if r.note.is_empty() { &r.from } else { &r.note }, if r.later.is_empty() { String::new() } else { format!(" · {}", r.later) })
    }).collect();
    let bad = problems(task);
    if let Some(t) = task.filter(|t| !t.arr("attempts").is_empty()) {
        lines.push(String::new());
        lines.push(format!("Attempts: {}", t.arr("attempts").iter().map(|a| format!("{} {}{}", a.s("letter"), a.s("agent"), if t.at("picked").s("letter") == a.s("letter") { " (chosen)" } else { "" })).collect::<Vec<_>>().join(" · ")));
    }
    let mut out = vec![head];
    out.extend(lines);
    if !bad.is_empty() { out.push(String::new()); out.push("Not counted (the next place down is used for each):".into()); out.extend(bad.iter().map(|b| format!("  {b}"))); }
    out.join("\n")
}
/// How a board says a value just saved or put back: plain, and `, for this task only` where it is that one's own now.
/// Where any other value comes from is `show`'s alone.
pub fn saved_words(label: &str, value: &str, from: &str) -> String {
    if from == "this task" || from == "this automation" { format!("{label}: {value}, for {from} only") } else { format!("{label}: {value}") }
}
