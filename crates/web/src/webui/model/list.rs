//! The task list for the web board: the groups and rows the terminal board shows, in its order, each row with the mark,
//! the words, the columns and the inks the board's own row code gives it (board/view/fields.rs). A machine's automations
//! are a group of its own, one row each, after its tasks' (by repo: in their repo's). The page draws what is here and
//! works nothing out.
use super::super::ink::{first, pieces};
use crate::board::state::{Row, State};
use crate::board::update::coords::REPO_HEAD_INK;
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
        "strong": false, "mark": m.s("t"), "markDim": m.b("d"), "markBold": m.b("b"), "color": status_ink(r).name(), "live": is_live(r),
        "says": pieces(&sentence(r)), "progress": if live { json!(r.f("progress").clamp(0.0, 100.0)) } else { Value::Null },
        "percent": f.percent.text().trim(), "eta": f.eta.text(), "review": pieces(&f.review), "pr": pieces(&f.pr), "prUrl": crate::review_page::render::web_url(r.s("pr")),
        "time": format!("{}{}", f.elapsed, if f.ago { " ago" } else { "" }), "quiet": (r.s("state") == "done"),
        "pinned": r.b("pinned"), "archived": r.b("archivedAt"), "app": r.s("app"), "made": r.at("automation").b("id"), // `made`: by an automation (its [auto] tag is dimmed, as every tag is)
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
/// The machines in view whose tasks could not be read, each with why and what to do: one notice over the list, which
/// says they do not answer, then a line each (`vm-a: cannot connect (it refused the connection).`). `asking`: its first
/// answer has not come yet, which is no fault.
pub fn unread(s: &State) -> Vec<Value> {
    let all = crate::board::update::remotes::in_view(s);
    all.into_iter().filter(|m| !m.ok).map(|m| match &m.why {
        Some(w) => {
            let mut d: Vec<String> = w.arr("detail").iter().map(text).collect();
            if w.b("said") { d.push(String::new()); d.push("What came back:".into()); d.extend(w.s("said").split('\n').map(|l| format!("  {l}"))); }
            json!({ "machine": m.machine.label, "line": format!("{}: {}.", m.machine.label, if w.b("line") { w.t("line") } else { "it did not answer".into() }), "detail": d, "asking": false })
        }
        None => json!({ "machine": m.machine.label, "line": format!("Asking {} for its tasks…", m.machine.label), "detail": [], "asking": true }),
    }).collect()
}
pub fn list_of(s: &State, t: &Theme) -> Value {
    let now = crate::clock::now();
    let mut groups: Vec<Obj> = vec![];
    let push = |groups: &mut Vec<Obj>, row: Value| { if let Some(g) = groups.last_mut() { if let Some(a) = g.get_mut("rows").and_then(|x| x.as_array_mut()) { a.push(row); } } };
    // A folded heading's count: how many rows it hides, in words where automations are among them (`7 tasks, 1 automation`).
    let count = |folded: usize, autos: usize| if autos > 0 { json!(crate::board::update::heads::holds(folded, autos)) } else { json!(folded) };
    // A machine's or a project's row: its coordinator's state as a task's mark, where it says something, and its time
    // (board/view/pilot.rs), and while on autopilot its status line (`under`), as a task's second line; null else.
    let pilot = |x: Option<crate::board::view::pilot::Shown>| x.map(|x| json!({ "mark": pieces(&x.mark), "label": x.label, "cols": if x.time.is_empty() { vec![] } else { vec![pieces(&x.time)] }, "under": pieces(&x.under) })).unwrap_or(Value::Null);
    let coord = |id: &str| crate::board::update::coords::of_head(s, id).is_some();
    // How far in each row stands (board/update/depth.rs): all machines → machine → project → task.
    let deep = crate::board::update::depth::depths(&s.rows);
    let top = crate::board::update::top::top_id(s);
    for (at, row) in s.rows.iter().enumerate() {
        let depth = deep.get(at).copied().unwrap_or(0);
        let deep_row = |mut v: Value| { if let Some(o) = v.as_object_mut() { o.insert("depth".into(), json!(depth)); } v };
        match row {
            Row::Head { name, id, autos, machine, folded, folded_autos, empty } => {
                // What the terminal board draws after the caret, in its inks (board/view/list.rs: heading_words): the name
                // in the base colour (a machine's and the top row's bold), a status group's in its colour, what goes with
                // it dimmed (a repo's key, what a folded row holds, what the top row holds).
                let (words, shown) = crate::board::view::list::heading_words(s, t, &s.rows, at);
                let repo = !*autos && (s.by_repo || id.starts_with(&format!("{}repo\t", crate::board::update::heads::HEAD)));
                let color = if *autos { "auto" } else if *machine || *id == top || repo { REPO_HEAD_INK.name() } else if ["needs you", "ready to approve", "working", "to do", "done"].contains(&name.as_str()) { group_ink(name).name() } else { "subtext0" };
                groups.push(obj(json!({ "name": if repo { repo_label(name) } else { name.clone() }, "key": if repo { crate::board::tags::heading_key(s, at) } else { String::new() }, "id": id,
                    "folded": *folded > 0 || *empty, "count": if *empty { json!("0 tasks") } else { count(*folded, *folded_autos) }, "color": color, "repo": repo, "machine": *machine, "top": *id == top, "automations": *autos,
                    "words": pieces(&words), "depth": depth, "coord": coord(id), "pilot": pilot(shown), "rows": [], "more": 0 })));
            }
            Row::Auto(a) => push(&mut groups, deep_row(auto_row_of(s, t, a, now))),
            Row::Rec(r) => push(&mut groups, deep_row(row_of(s, t, r, now))),
            Row::Note(n) => if let Some(g) = groups.last_mut() { g.put("more", crate::re!(r"(\d+) done").captures(n).and_then(|c| c[1].parse::<i64>().ok()).unwrap_or(0)); },
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
    let empty: String = if !s.rows.is_empty() || (!unread.is_empty() && s.scope != "*") { String::new() } else if !s.filter.is_empty() { "Nothing matches.".into() } else if s.archive_view { "No archived tasks.".into() } else if s.pinned_view { crate::board::update::pinned::empty_says() } else { "Nothing here yet. Type what needs doing.".into() };
    json!({
        "head": {
            // The repo new tasks go to, by its name: no folder path, and nothing for the home folder.
            "where": if s.archive_view || s.data.repo_root == home { String::new() } else { s.data.repo.clone() },
            "whereKey": if s.archive_view || s.data.repo_root == home { String::new() } else { s.data.keys.get(&s.data.repo).cloned().unwrap_or_default() },
            "session": if s.data.session == "default" { String::new() } else { s.data.session.clone() },
            "machines": if s.scope == "*" { let l: Vec<String> = crate::board::update::remotes::others(s).into_iter().map(|m| { let n = crate::board::update::remotes::shown(s, &m.machine.label); if m.ok { n } else { format!("{n} (no answer)") } }).collect(); if l.is_empty() { "no others found".to_string() } else { l.join(", ") } } else { String::new() },
            "only": if s.all() && s.scope != "*" { s.scope.clone() } else { String::new() }, "filter": s.filter, "host": crate::remote::check::hostname(), "version": crate::store::files::VERSION,
            // The tabs, as the terminal board's header has them (view/tabs.rs): which shows, and how many rows need you in
            // each, so the page says what needs you in the tab not shown.
            "tabs": { "pinned": s.pinned_view && !s.archive_view, "board": s.tab_needs.0, "pinnedNeeds": s.tab_needs.1 },
        },
        // The filter row under the header (shows.rs).
        "shows": super::shows::shows_of(s),
        "groups": groups, "selected": s.sel,
        // Words on their way to another machine: rows of their own under the list, until that machine has taken them.
        "jobs": s.jobs.iter().map(|j| json!({ "id": j.s("id"), "text": j.s("text"), "failed": j.s("state") == "error", "msg": j.s("msg") })).collect::<Vec<_>>(),
        "unread": unread, "empty": empty,
    })
}
