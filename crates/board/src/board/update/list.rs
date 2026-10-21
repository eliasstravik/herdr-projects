//! The task list from what was read: which rows there are, under which headings, which one is selected. The selection
//! stays on its task while other rows move; when its own task moves (to another group, elsewhere in the list, out of
//! it) the selection stays where you were working (landing.rs). Pure: what was read comes in with the state (data.rs).
use super::heads::{autos_in, group_id, host_of, is_head, machine_id, parts};
use super::landing::{land, AUTOS, JOBS};
use crate::board::state::{Away, Mode, Rec, Row, SelRow, State};
use crate::board::tags::number_tag;
use crate::board::keys::key_name;
use crate::json::*;
use crate::shown::ink::Ink;
use crate::shown::queue::queued;
use crate::shown::states::{group_of, is_live, GROUPS};

/// A long done list folds to its latest few. A list taller than its room shows fewer still, down to one.
const DONE_SHOWN: usize = 5;
pub const AUTOMATIONS: &str = "automations";
fn is_done(r: &Obj) -> bool { r.s("state") == "done" && !r.b("pinned") && !r.b("archivedAt") }
fn is_todo(r: &Obj) -> bool { group_of(r) == "to do" && !r.b("pinned") && !r.b("archivedAt") }
fn rank(r: &Obj) -> i64 { if r.b("archivedAt") { -2 } else if r.b("pinned") { -1 } else { GROUPS.iter().position(|g| *g == group_of(r)).map(|n| n as i64).unwrap_or(9) } }
fn repo_of(id: &str) -> &str { id.split('/').next().unwrap_or("") }
/// A task's repo as its heading names it: another machine's by its own id there, so its machine is not in it.
fn repo_name(r: &Obj) -> &str { repo_of(if r.b("rid") { r.s("rid") } else { r.s("id") }) }

/// The filter as typed (find) or kept.
pub fn filter_now(s: &State) -> String { (if s.mode == Mode::Find { s.value() } else { s.filter.clone() }).trim().to_lowercase() }
/// A task's tags as find takes them: `hsw-134`, `#134`, `134`.
fn tags(s: &State, r: &Obj) -> Vec<String> {
    let tag = number_tag(r, &s.data.keys, &s.scope, false).to_lowercase();
    if tag.is_empty() { return vec![]; }
    let n = tag.rsplit(['#', '-']).next().unwrap_or("").to_string();
    vec![tag, format!("#{n}"), n]
}
/// The automations of whoever's board shows, machine by machine in the order of herdr's sidebar. None in the archive.
/// Each stands in its machine's part of the list: its own group there, or under its repo (heads.rs: autos_in).
pub fn autos_shown(s: &State, f: &str) -> Vec<Obj> {
    if s.archive_view { return vec![]; }
    let mut all: Vec<Obj> = if s.scope.is_empty() || s.scope == "*" { s.data.autos.iter().map(|a| { let mut a = a.clone(); let row = format!("auto:{}", a.s("id")); a.put("row", row); a.put("host", ""); a }).collect() } else { vec![] };
    for m in super::remotes::in_view(s) { all.extend(m.autos.iter().cloned()); }
    all.sort_by_key(|a| super::remotes::place(s, a.s("host")));
    if f.is_empty() { return all; }
    all.into_iter().filter(|a| { let r = a.at("rowNow"); format!("automation {} {} {} {} {}", r.s("name"), r.s("trigger"), r.s("repo"), r.s("state"), a.s("host")).to_lowercase().contains(f) }).collect()
}
/// What the list is made of, from the items: call after anything changed what shows (a load, a key, a filter).
pub fn relist(s: &mut State) {
    super::remotes::reorder(s);
    let mine = &s.data.items;
    s.items = if s.scope.is_empty() { mine.clone() } else if s.scope == "*" { mine.iter().cloned().chain(super::remotes::others(s).into_iter().flat_map(|m| m.items.iter().cloned())).collect() } else { s.remotes.get(&s.scope).map(|m| m.items.clone()).unwrap_or_default() };
    super::pending::overlay(s);
    s.jobs = if s.archive_view { vec![] } else { s.data.jobs.iter().filter(|j| !super::chat::in_chat(s, j)).cloned().collect() };
    let f = filter_now(s);
    s.autos = autos_shown(s, &f);
    super::pending::overlay_autos(s);
    let layout = serde_json::to_string(&json!([s.scope, s.by_repo, s.all_done, s.archive_view, f])).unwrap_or_default();
    // The task the selection is held on (or that the panel shows whole, in the same list) keeps its row whatever it has
    // become meanwhile.
    let held = |r: &Obj| r.s("id") == s.sel && (r.s("id") == s.stick || (s.full && layout == s.selection_layout));
    let eligible: Vec<Rec> = s.items.iter().filter(|r| held(r) || if s.archive_view { r.b("archivedAt") } else { !f.is_empty() || !r.b("archivedAt") }).cloned().collect();
    let shown: Vec<Rec> = if f.is_empty() { eligible } else { eligible.into_iter().filter(|r| held(r) || {
        let hay = format!("{} {} {} {} {} {} {} {} {} {}", tags(s, r).join(" "), r.s("id"), r.s("title"), r.s("msg"), r.t("state"), r.s("app"), if r.b("archivedAt") { "archived" } else { "" }, r.s("description"), r.at("failure").s("title"), r.at("failure").s("detail"));
        hay.to_lowercase().contains(&f)
    }).collect() };
    s.grouped = if s.by_repo {
        let mut names: Vec<String> = vec![];
        for r in &shown { let g = repo_name(r).to_string(); if !names.contains(&g) { names.push(g); } }
        for a in &s.autos { let g = a.t("repo"); if !names.contains(&g) { names.push(g); } }
        // A repo used lately with no task in view keeps its heading, for its project coordinator (heads.rs: empty_repo).
        if s.scope.is_empty() || s.scope == "*" { for g in &s.data.lately { if super::heads::empty_repo(s, "", g) && !names.contains(g) { names.push(g.clone()); } } }
        let here = s.data.repo.clone();
        names.sort_by(|a, b| if *a == here { std::cmp::Ordering::Less } else if *b == here { std::cmp::Ordering::Greater } else { a.to_lowercase().cmp(&b.to_lowercase()).then(b.cmp(a)) });
        names.into_iter().map(|g| { let list = shown.iter().filter(|r| repo_name(r) == g).cloned().collect(); (g, list) }).collect()
    } else {
        let mut g: Vec<(String, Vec<Rec>)> = vec![("archived".to_string(), shown.iter().filter(|r| r.b("archivedAt")).cloned().collect::<Vec<_>>()), ("pinned".to_string(), shown.iter().filter(|r| r.b("pinned") && !r.b("archivedAt")).cloned().collect())];
        for name in GROUPS {
            let mut rows: Vec<Rec> = shown.iter().filter(|r| group_of(r) == name && !r.b("pinned") && !r.b("archivedAt")).cloned().collect();
            // The queued tasks stand last under `working`: those that wait for a free slot in the order they get one
            // (Q#1 first), then those queued after other tasks.
            rows.sort_by_key(|r| if r.s("stage") == "queued" && queued(r) { (1, r.i("queuePlace")) } else if queued(r) { (2, 0) } else { (0, 0) });
            g.push((name.to_string(), rows));
        }
        // A machine's automations are a group of their own, after its tasks' (fold_rows gives each its own).
        g.push((AUTOMATIONS.to_string(), vec![]));
        g
    };
    s.whole = s.all_done || !f.is_empty();
    // What is typed is a task's own number or tag: the selection goes to that task.
    if !f.is_empty() && layout != s.selection_layout { if let Some(exact) = shown.iter().find(|r| tags(s, r).contains(&f)) { s.sel = exact.t("id"); } }
    // The selection taken to a task (or the list laid out anew under it): that task's headings unfold.
    if !is_head(&s.sel) && (layout != s.selection_layout || !s.selection_rows.iter().any(|r| r.id == s.sel)) { super::heads::reveal(s); }
    s.rows = fold_rows(s);
    // Every row the arrows stop on (the to-dos a long list folds too), with the heading it stands under.
    let mut under: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (name, recs) in &s.grouped { for r in recs { let host = if s.scope == "*" { host_of(r) } else { s.scope.clone() }; under.insert(r.t("id"), group_id(s, &host, name)); } }
    let recs: std::collections::HashMap<&str, &Rec> = s.items.iter().map(|r| (r.s("id"), r)).collect();
    let autos: std::collections::HashMap<&str, &Obj> = s.autos.iter().map(|a| (a.s("row"), a)).collect();
    // An automation's row stands under its machine's automations (landing.rs: AUTOS), so that one removed lands on its kin.
    let selectable: Vec<(Rec, String)> = s.steps.iter().filter_map(|id| if is_head(id) { Some((Rec::new(obj(json!({ "id": id, "state": "heading" }))), id.clone())) }
            else if let Some(a) = autos.get(id.as_str()) { Some((Rec::new(obj(json!({ "id": id, "state": "automation", "created": a.at("created") }))), format!("{AUTOS}\t{}", a.s("host")))) }
            else { recs.get(id.as_str()).map(|r| ((*r).clone(), under.get(id).cloned().unwrap_or_default())) })
        .chain(s.jobs.iter().cloned().map(|j| (Rec::new(j), JOBS.to_string()))).collect();
    let folded_heads: Vec<String> = s.rows.iter().filter_map(|r| match r { Row::Head { id, .. } if r.shut() => Some(id.clone()), _ => None }).collect();
    // Only the selected task's ordering data matters: new rows, progress and other tasks moving never steal the selection.
    let base: std::collections::HashMap<String, i64> = s.items.iter().enumerate().map(|(n, r)| (r.t("id"), n as i64)).collect();
    let next: Vec<SelRow> = selectable.iter().map(|(r, group)| { let task = r.has("title"); SelRow { group: group.clone(),
        // Where the row stands, not what the task is: a pinned (or archived) task keeps its row whatever its status.
        id: r.t("id"), order: serde_json::to_string(&json!([if task { rank(r).to_string() } else { r.t("state") }, r.b("pinned"), r.at("created"), r.b("archivedAt")])).unwrap_or_default(),
        at: if task && is_done(r) { r.i("at") } else { 0 }, done: task && is_done(r), base: base.get(r.s("id")).copied().unwrap_or(-1) } }).collect();
    let ids: Vec<String> = next.iter().map(|r| r.id.clone()).collect();
    let selected = s.sel.clone();
    let before = s.selection_rows.iter().position(|r| r.id == selected);
    let after_n = ids.iter().position(|i| *i == selected);
    let after = after_n.map(|n| next[n].clone());
    let old = before.map(|n| s.selection_rows[n].clone());
    let crossed = match (&old, &after, before, after_n) {
        (Some(o), Some(a), Some(b), Some(an)) if o.done && a.done && o.at != a.at => s.selection_rows.iter().enumerate().any(|(n, r)| {
            let Some(to) = ids.iter().position(|i| *i == r.id) else { return false };
            if r.id == selected || !r.done || (s.by_repo && repo_of(&r.id) != repo_of(&selected)) { return false; }
            let would = r.at > a.at || (r.at == a.at && r.base < o.base);
            (n < b) != would && (n < b) != (to < an)
        }),
        _ => false,
    };
    // The selection stays on its task while you read it whole or answer it, and while you are in its agent. Else a task
    // that moves (an action you took on it, or its agent) leaves the selection where you were working.
    let reading = s.reply || ((s.full || s.stick == selected) && after.is_some());
    if layout == s.selection_layout && old.is_some() && !reading && (after.is_none() || after.as_ref().map(|a| a.order != old.as_ref().unwrap().order).unwrap_or(false) || crossed) {
        s.sel = land(&s.selection_rows, before.unwrap(), &ids, &|id| folded_heads.iter().any(|h| h == id)).unwrap_or_else(|| if after.is_some() { selected.clone() } else { first_row(&ids) });
    }
    if !ids.contains(&s.sel) { s.sel = first_row(&ids); }
    s.selection_rows = next;
    s.selection_layout = layout;
    // With no automation selected, nothing of one is open.
    if s.auto().is_none() { s.auto_open = false; s.auto_at = -1; if s.editing.as_ref().map(|e| e.whose == crate::board::state::Whose::Auto).unwrap_or(false) { s.editing = None; } }
    // A question is about what it names: when that changed underneath it, the question goes.
    if let Some(a) = &s.ask { if !super::ask::still_valid(s, &a.valid) { s.ask = None; s.flash(format!("It changed while you were asked, so nothing was merged. Look at it again, then {}.", key_name("yes")), Ink::Yellow); } }
    // A reply belongs to the task it was started for: when that task is gone, the words are put away, nothing is sent.
    if s.reply && s.mode == Mode::New && !s.value().is_empty() && s.sel != s.reply_id && (!s.reply_id.is_empty() || s.current().is_some()) {
        if !s.value().trim().is_empty() { s.draft_away = Some(Away { text: s.value(), to: s.reply_id.clone(), cwd: s.ctx.cwd.clone(), warned: false }); }
        s.input.set_text(""); s.full = false;
        if !s.reply_id.is_empty() { s.reply = false; s.flash("The task you were replying to is gone. Nothing was sent; your words are put away.", Ink::Yellow); }
        else { s.flash("The row you were on is gone. Nothing was sent; your words for the Coordinator are put away.", Ink::Yellow); }
    }
    sync_reply(s);
    if s.sel != selected { s.rows = fold_rows(s); } // the done fold for the new selection
}
/// Where a selection with nowhere else to go lands: the first task (or automation), a heading only where there is none,
/// never the main Coordinator's row on top (it is reached with ↑).
fn first_row(ids: &[String]) -> String { ids.iter().find(|i| !is_head(i)).or(ids.iter().find(|i| !super::pilot::is_main(i))).cloned().unwrap_or_default() }
/// While you talk to a task's agent the chat follows the selection; the task you talk to going away is back to the
/// Coordinator, never on to a neighbour's agent. On a row with no task, `reply_id` is empty.
pub fn sync_reply(s: &mut State) {
    if !s.reply || s.sel == s.reply_id { return; }
    if !s.reply_id.is_empty() && !super::coords::exists(s, &s.reply_id) { s.reply = false; s.full = false; s.panel_top = 0; }
    else { s.reply_id = if s.current().is_some() || super::coords::of_head(s, &s.sel).is_some() { s.sel.clone() } else { String::new() }; s.panel_top = 0; }
}
/// The rows under their headings. A long done list folds to its latest few; a list taller than its room shows fewer
/// done rows, down to one, then fewer to-dos of a list that has live tasks. The selected task is never folded away,
/// nor anything while you search or ask for all done. A machine's automations stand in its part, after its tasks (by
/// repo: after their repo's), and fold with their heading.
pub fn fold_rows(s: &mut State) -> Vec<Row> {
    s.fold_sel = s.sel.clone();
    let groups = s.grouped.clone();
    let parts = parts(s, &groups);
    let lay = |keep: usize, todos: usize, s: &State| -> Vec<Row> {
        let mut rows = vec![];
        // The main Coordinator's row, on top: its coordinator's, as every heading's is (update/pilot.rs).
        if let Some(id) = super::pilot::main_row(s) { rows.push(Row::Head { name: crate::status_words::COORDINATOR.into(), autos: false, id, machine: false, folded: 0, folded_autos: 0, empty: false }); }
        for p in &parts {
            let autos: Vec<Vec<Obj>> = p.groups.iter().map(|(name, _)| autos_in(s, &p.host, name)).collect();
            let n_autos: usize = autos.iter().map(|a| a.len()).sum();
            let n: usize = p.groups.iter().map(|(_, recs)| recs.len()).sum::<usize>() + n_autos;
            let bare = |name: &str, recs: &[Rec], autos: &[Obj]| recs.is_empty() && autos.is_empty() && super::heads::empty_repo(s, &p.host, name);
            let has_bare = p.groups.iter().zip(&autos).any(|((name, recs), a)| bare(name, recs, a));
            // A machine's heading, with every machine shown: folded, its one line stands for all its groups. One with
            // nothing in view shows folded (`0 tasks`), so that its machine coordinator is still a space away.
            if p.heading && (n > 0 || has_bare || super::heads::empty_shown(s)) {
                let id = machine_id(&p.host);
                // Only empty repos' headings under it: nothing to fold.
                let shut = super::heads::folded(s, &id) && n > 0;
                rows.push(Row::Head { name: super::remotes::shown(s, &p.host), autos: false, id, machine: true, folded: if shut { n } else { 0 }, folded_autos: if shut { n_autos } else { 0 }, empty: n == 0 && !has_bare });
                if shut { continue; }
            }
            if n == 0 && !has_bare { continue; }
            for ((name, recs), autos) in p.groups.iter().zip(&autos) {
                // A repo used lately with nothing in view: its heading alone, folded (`0 tasks`).
                let empty = bare(name, recs, autos);
                if recs.is_empty() && autos.is_empty() && !empty { continue; }
                let id = group_id(s, &p.host, name);
                let shut = super::heads::folded(s, &id) && !empty;
                let own = !s.by_repo && name == AUTOMATIONS;
                rows.push(Row::Head { name: name.clone(), autos: own, id, machine: false, folded: if shut { recs.len() + autos.len() } else { 0 }, folded_autos: if shut { autos.len() } else { 0 }, empty });
                if shut || empty { continue; }
                let mut list = recs.clone();
                list.sort_by(|a, b| (if s.by_repo { rank(a).cmp(&rank(b)) } else { std::cmp::Ordering::Equal })
                    .then_with(|| if a.b("archivedAt") && b.b("archivedAt") { b.i("archivedAt").cmp(&a.i("archivedAt")) } else { std::cmp::Ordering::Equal })
                    .then_with(|| if is_done(a) && is_done(b) { b.i("at").cmp(&a.i("at")) } else { std::cmp::Ordering::Equal }));
                let done: Vec<&Rec> = list.iter().filter(|r| is_done(r)).collect();
                let folded: Vec<String> = if s.whole { vec![] } else { done.iter().skip(keep).filter(|r| r.s("id") != s.sel).map(|r| r.t("id")).collect() };
                let todo: Vec<&Rec> = list.iter().filter(|r| is_todo(r)).collect();
                let from = match todo.iter().position(|r| r.s("id") == s.sel) { Some(p) if todos <= p => p + 1 - todos, _ => 0 };
                let later: Vec<String> = if s.whole { vec![] } else { todo.iter().enumerate().filter(|(n, _)| *n < from || *n >= from.saturating_add(todos)).map(|(_, r)| r.t("id")).collect() };
                let last_done = done.last().map(|r| r.t("id"));
                let last_todo = todo.last().map(|r| r.t("id"));
                for r in &list {
                    let id = r.t("id");
                    if !folded.contains(&id) && !later.contains(&id) { rows.push(Row::Rec(r.clone())); }
                    if !folded.is_empty() && Some(&id) == last_done.as_ref() { rows.push(Row::Note(format!("↓ {} more done ({} shows them)", folded.len(), key_name("allDone")))); }
                    if !later.is_empty() && Some(&id) == last_todo.as_ref() { rows.push(Row::Note(format!("↓ {} more to do", later.len()))); }
                }
                rows.extend(autos.iter().cloned().map(Row::Auto));
            }
        }
        rows
    };
    // No task gives way for an automation: theirs are not counted.
    let lines_of = |rows: &[Row], s: &State| -> usize { rows.iter().enumerate().map(|(n, r)| match r { Row::Head { .. } => heading_height(rows, n) + crate::board::view::pilot::extra(s, r, s.list_width), Row::Rec(o) => crate::board::view::row_wrap::row_height(s, o, s.list_width, s.list_tall), Row::Auto(_) => 0, _ => 1 }).sum() };
    let over = |rows: &[Row], s: &State| !s.whole && s.list_room > 0 && lines_of(rows, s) > s.list_room;
    let mut keep = DONE_SHOWN;
    let mut rows = lay(keep, usize::MAX, s);
    while keep > 1 && over(&rows, s) { keep -= 1; rows = lay(keep, usize::MAX, s); }
    // The arrow keys go through every heading and every to-do, folded or not.
    s.steps = rows.iter().filter_map(step_id).collect();
    let mut todos = groups.iter().map(|(_, recs)| recs.iter().filter(|r| is_todo(r)).count()).max().unwrap_or(0);
    let live = rows.iter().any(|r| matches!(r, Row::Rec(o) if is_live(o)));
    while live && todos > 1 && over(&rows, s) { todos -= 1; rows = lay(keep, todos, s); }
    rows
}
/// The lines the heading at `n` takes: an empty line above it, but for a group's heading right under its machine's.
pub fn heading_height(rows: &[Row], n: usize) -> usize { if n > 0 && matches!(rows[n - 1], Row::Head { machine: true, folded: 0, empty: false, .. }) { 1 } else { 2 } }
/// A row the arrow keys stop on: a task, an automation or a heading.
fn step_id(r: &Row) -> Option<String> { match r { Row::Rec(o) => Some(o.t("id")), Row::Auto(a) => Some(a.t("row")), Row::Head { id, .. } if !id.is_empty() => Some(id.clone()), _ => None } }
/// Up or down `d` rows: through every task (folded to-dos too) and automation, then words on their way.
pub fn step(s: &mut State, d: i64) {
    let mut ids: Vec<String> = if s.steps.is_empty() { s.rows.iter().filter_map(step_id).collect() } else { s.steps.clone() };
    ids.extend(s.jobs.iter().map(|j| j.t("id")));
    let at = ids.iter().position(|i| *i == s.sel).map(|n| n as i64).unwrap_or(-1);
    // No task to move to: what the list says instead (why a machine does not answer) scrolls.
    let has_rec = s.rows.iter().any(|r| matches!(r, Row::Rec(_)));
    if !has_rec && s.unread_max > 0 && (if d > 0 { s.unread_top < s.unread_max } else { s.unread_top > 0 && at <= 0 }) { s.unread_top = (s.unread_top as i64 + d * 3).clamp(0, s.unread_max as i64) as usize; return; }
    if !ids.is_empty() { s.sel = ids[(at + d).clamp(0, ids.len() as i64 - 1) as usize].clone(); }
    s.rows = fold_rows(s);
}
/// How many tasks stand in each group, for the header: archived ones apart.
pub fn counts(s: &State) -> std::collections::BTreeMap<&'static str, usize> {
    let mut n = std::collections::BTreeMap::new();
    for r in &s.items { *n.entry(if r.b("archivedAt") { "archived" } else { group_of(r) }).or_insert(0) += 1; }
    n
}
