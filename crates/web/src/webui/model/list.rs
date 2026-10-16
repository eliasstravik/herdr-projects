//! The task list for the web board: the groups and rows the terminal board shows, in its order, each row with the mark,
//! the words, the columns and the inks the board's own row code gives it (board/view/fields.rs). A machine's automations
//! are a group of its own, one row each, after its tasks' (by repo: in their repo's). The page draws what is here and
//! works nothing out.
use super::super::ink::{first, pieces};
use crate::board::state::{Row, State};
use crate::board::tags::{row_tag, tag_cells};
use crate::board::theme::Theme;
use crate::board::view::fields::{fields, mark, row_title};
use crate::board::view::row::list_numbering;
use crate::json::*;
use crate::shown::row_says::{row_fields, sentence};
use crate::shown::states::{group_ink, is_live, status_ink};
use crate::shown::words::{id_label, repo_label};

pub fn row_of(s: &State, t: &Theme, r: &Obj, now: i64) -> Value {
    let f = row_fields(r, now);
    let m = first(&mark(t, r));
    let live = r.s("state") != "done" && r.at("progress").is_number();
    json!({
        "id": r.s("id"), "label": id_label(if r.b("rid") { r.s("rid") } else { r.s("id") }), "tag": row_tag(r, &s.data.keys), "host": r.s("host"), "title": row_title(s, r),
        "strong": (["working", "blocked", "error"].contains(&r.s("state"))), "mark": m.s("t"), "markDim": m.b("d"), "markBold": m.b("b"), "color": status_ink(r).name(), "live": is_live(r),
        "says": pieces(&sentence(r)), "progress": if live { json!(r.f("progress").clamp(0.0, 100.0)) } else { Value::Null },
        "percent": f.percent.text().trim(), "eta": f.eta.text(), "review": pieces(&f.review), "pr": pieces(&f.pr), "prUrl": crate::review_page::render::web_url(r.s("pr")),
        "time": format!("{}{}", f.elapsed, if f.ago { " ago" } else { "" }), "quiet": (r.s("state") == "done"),
        "pinned": r.b("pinned"), "archived": r.b("archivedAt"), "app": r.s("app"), "made": r.at("automation").b("id"), // `made`: by an automation, whose colour its [auto] tag takes
    })
}
/// One automation's row, in a task row's places: the word on or off where a task has its number, its trigger (and why
/// it was turned off) where a task says something, the task its last firing made among the tags, and "next in 4m" or
/// "fired 2h ago" where a task has its time. Off, it is quiet as a done task is.
pub fn auto_row_of(s: &State, t: &Theme, a: &Obj, now: i64) -> Value {
    let rec = obj(json!({ "id": a.s("row"), "auto": a }));
    let f = fields(s, t, &rec, now);
    let m = first(&mark(t, &rec));
    json!({ "id": a.s("row"), "label": "", "tag": row_tag(&rec, &s.data.keys), "host": a.s("host"), "title": row_title(s, &rec), "strong": false, "mark": m.s("t"), "markDim": !a.b("on"), "markBold": false, "color": "auto", "live": false,
        "says": pieces(&crate::board::view::fields::auto_said(s, &rec, "all")), "progress": Value::Null, "percent": "", "eta": "", "review": pieces(&f.tag), "pr": [], "prUrl": "", "time": f.when, "quiet": !a.b("on"),
        "pinned": false, "archived": false, "app": "", "automation": true }) // `automation`: its word (on, off) never gives way, and what it says is never clamped (board.css)
}
/// The machines in view whose tasks could not be read, each with why and what to do: said where their tasks would be.
pub fn unread(s: &State) -> Vec<Value> {
    let all = crate::board::update::remotes::in_view(s);
    all.into_iter().filter(|m| !m.ok).map(|m| match &m.why {
        Some(w) => {
            let mut d: Vec<String> = vec![];
            if w.b("said") { d.push("What came back:".into()); d.extend(w.s("said").split('\n').map(|l| format!("  {l}"))); d.push(String::new()); }
            d.extend(w.arr("detail").iter().map(text));
            json!({ "machine": m.machine.label, "line": format!("{}: its tasks cannot be read: {}.", m.machine.label, w.s("line")), "detail": d })
        }
        None => json!({ "machine": m.machine.label, "line": format!("{}: asking for its tasks…", m.machine.label), "detail": [] }),
    }).collect()
}
pub fn list_of(s: &State, t: &Theme) -> Value {
    let now = crate::clock::now();
    let mut groups: Vec<Obj> = vec![];
    let push = |groups: &mut Vec<Obj>, row: Value| { if let Some(g) = groups.last_mut() { if let Some(a) = g.get_mut("rows").and_then(|x| x.as_array_mut()) { a.push(row); } } };
    // A folded heading's count: how many rows it hides, in words where automations are among them (`7 tasks, 1 automation`).
    let count = |folded: usize, autos: usize| if autos > 0 { json!(crate::board::update::heads::holds(folded, autos)) } else { json!(folded) };
    for (at, row) in s.rows.iter().enumerate() {
        match row {
            Row::Head { name, id, autos: true, folded, folded_autos, .. } => groups.push(obj(json!({ "name": name, "id": id, "folded": *folded > 0, "count": count(*folded, *folded_autos), "color": "auto", "repo": false, "automations": true, "rows": [], "more": 0 }))),
            // A machine's heading stands over its groups: one with no rows, that a tap folds and unfolds as a group's.
            Row::Head { name, id, machine: true, folded, folded_autos, .. } => groups.push(obj(json!({ "name": name, "id": id, "machine": true, "folded": *folded > 0, "count": count(*folded, *folded_autos), "color": "accent", "repo": false, "rows": [], "more": 0 }))),
            Row::Head { name, id, folded, folded_autos, .. } => groups.push(obj(json!({ "name": if s.by_repo { repo_label(name) } else { name.clone() }, "key": if s.by_repo { crate::board::tags::heading_key(s, at) } else { String::new() }, "id": id, "folded": *folded > 0, "count": count(*folded, *folded_autos), "color": if s.by_repo || !["needs you", "ready to approve", "working", "to do", "done"].contains(&name.as_str()) { "subtext0" } else { group_ink(name).name() }, "repo": s.by_repo, "rows": [], "more": 0 }))),
            Row::Auto(a) => push(&mut groups, auto_row_of(s, t, a, now)),
            Row::Rec(r) => push(&mut groups, row_of(s, t, r, now)),
            Row::Note(n) => if let Some(g) = groups.last_mut() { g.put("more", crate::re!(r"(\d+) more").captures(n).and_then(|c| c[1].parse::<i64>().ok()).unwrap_or(0)); },
        }
    }
    // The characters each task's tag (`#134`, `HSW-134`) fills: the terminal board's column and its limit on the space
    // before a title (tags.rs), so the page lines the titles up the same way.
    let column = list_numbering(s, &[]).cells;
    for g in groups.iter_mut() {
        if let Some(rows) = g.get_mut("rows").and_then(|x| x.as_array_mut()) {
            for r in rows.iter_mut() {
                let n = r.s("tag").chars().count();
                let w = if r.b("automation") { column.max(n) } else { let c = tag_cells(n, column); if c == 0 { n } else { c } };
                if let Some(o) = r.as_object_mut() { o.insert("tagWidth".into(), json!(w)); }
            }
        }
    }
    let home = crate::store::files::user_home().display().to_string();
    let unread = unread(s);
    let empty = if !s.rows.is_empty() || (!unread.is_empty() && s.scope != "*") { "" } else if !s.filter.is_empty() { "Nothing matches." } else if s.archive_view { "No archived tasks." } else { "Nothing here yet. Type what needs doing." };
    json!({
        "head": {
            // The repo new tasks go to, by its name: no folder path, and nothing for the home folder.
            "where": if s.archive_view || s.data.repo_root == home { String::new() } else { s.data.repo.clone() },
            "whereKey": if s.archive_view || s.data.repo_root == home { String::new() } else { s.data.keys.get(&s.data.repo).cloned().unwrap_or_default() },
            "session": if s.data.session == "default" { String::new() } else { s.data.session.clone() },
            "machines": if s.scope == "*" { let l: Vec<String> = crate::board::update::remotes::others(s).into_iter().map(|m| if m.ok { m.machine.label.clone() } else { format!("{} (no answer)", m.machine.label) }).collect(); if l.is_empty() { "no others found".to_string() } else { l.join(", ") } } else { String::new() },
            "only": if s.all() && s.scope != "*" { s.scope.clone() } else { String::new() }, "filter": s.filter, "host": crate::remote::check::hostname(), "version": crate::store::files::VERSION,
        },
        "groups": groups, "selected": s.sel,
        // Words on their way to another machine: rows of their own under the list, until that machine has taken them.
        "jobs": s.jobs.iter().map(|j| json!({ "id": j.s("id"), "text": j.s("text"), "failed": j.s("state") == "error", "msg": j.s("msg") })).collect::<Vec<_>>(),
        "unread": unread, "empty": empty,
    })
}
