//! The task list as drawn: the header line, the rows under their headings, the selected row's highlight, and the last
//! line of a list longer than its room, which says how many tasks are not shown. Also where each line's task is, for a
//! click.
use super::row::{row_layout, row_lines};
use super::row_tall::tall_at;
use crate::board::keys::key_name;
use crate::board::state::{Mode, Rec, Row, State};
use crate::board::theme::Theme;
use crate::board::update::coords::{MACHINE_INK, REPO_HEAD_INK};
use crate::board::update::heads::is_head;
use crate::board::update::list::{counts, fold_rows, heading_height};
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::shown::states::group_ink;
use crate::shown::words::repo_label;

/// The selected row, in every list: a bar in the gutter of each of its lines and the theme's tint behind it; where no
/// background can be painted, the line in reverse video instead: the selection never hangs on colour alone.
pub fn hl(t: &Theme, line: &Inked, width: usize) -> Inked {
    let cut = line.fit(width);
    let barred = if cut.text().starts_with(' ') { Inked::of("▌", Ink::Accent).add(cut.skip(1)) } else { cut };
    if t.paints_selection() { barred.on(Ink::Selection) } else { barred.inv() }
}
/// The header says where you are: the repo new tasks go to (whose board shows is the filter row's first item). Under it the filter row, with how many tasks each holds (shows_row.rs), where the list part has `h` lines enough
/// for it; a find has a line of its own under that.
pub fn head_lines(s: &mut State, t: &Theme, w: usize, h: usize) -> Vec<Inked> {
    // Whose board shows is the filter row's first item (shows_row.rs), not the header's.
    let scope = String::new();
    let n = counts(s);
    let c = |k: &str| n.get(k).copied().unwrap_or(0);
    // The counts of each status are on the filter row; the archive has none, so its count stays here.
    // The repo with its key beside it (`gtm-agent · AGT`), in the header's quiet ink as the rest of it.
    let where_ = if s.archive_view { "archive".to_string() } else { crate::board::tags::repo_with_key(&repo_label(&s.data.repo), s.data.keys.get(&s.data.repo).map(|k| k.as_str()).unwrap_or("")) };
    let projects = match s.data.archived.len() { 0 => String::new(), 1 => "1 project, ".into(), p => format!("{p} projects, ") };
    let tally: Vec<String> = if s.archive_view { vec![format!("{projects}{} archived", c("archived"))] } else { vec![] };
    let mut rest: Vec<String> = vec![];
    if s.scope.is_empty() || s.scope == "*" || s.archive_view { rest.push(where_); }
    rest.extend(tally);
    if !s.data.session.is_empty() && s.data.session != "default" { rest.push(format!("session {}", s.data.session)); }
    let rest = rest.join(" · ");
    // The board's name gives way before whose tasks these are; names too long for the line go on to the next, whole.
    let name = Inked::plain(" ").add(Inked::plain("herdr-projects").bold()).add(Inked::plain("  "));
    let lead = if !scope.is_empty() && name.width() + crate::text::width(&scope) > w { Inked::plain(" ") } else { name };
    let named: Vec<Inked> = if scope.is_empty() { vec![Inked::new()] } else { Inked::of(scope.clone(), Ink::Accent).wrap(w.saturating_sub(lead.width()).max(4)) };
    let first = lead.add(named[0].clone());
    let left = w as i64 - first.width() as i64 - if scope.is_empty() { 0 } else { 3 };
    let more: Vec<Inked> = named[1..].iter().map(|l| Inked::plain(" ").add(l.clone())).collect();
    let tail = if !rest.is_empty() && more.is_empty() && left >= 4 { Inked::of(format!("{}{}", if scope.is_empty() { "" } else { " · " }, crate::text::cut(&rest, left as usize)), Ink::Muted) } else { Inked::new() };
    let mut head = vec![first.add(tail)];
    head.extend(more);
    // In a window too short for it, the filter row gives way before the list's rows do.
    if !s.archive_view && h >= 6 { let (row, hits) = super::shows_row::shows_line(s, t, w); let y = head.len(); s.show_hits = hits.into_iter().map(|(a, b, n)| (y, a, b, n)).collect(); head.push(row); } else { s.show_hits.clear(); }
    let f = (if s.mode == Mode::Find { s.value() } else { s.filter.clone() }).trim().to_string();
    if f.is_empty() { return head; }
    let found: usize = s.grouped.iter().map(|(_, r)| r.len()).sum();
    head.push(Inked::plain(" ").add(Inked::of(crate::text::cut(&format!("filter \"{f}\" · {found} of {} · {} {}", s.items.len(), key_name("back"), if s.mode == Mode::Find { "cancels" } else { "clears" }), w.saturating_sub(1).max(1)), Ink::Accent)));
    head
}
/// The machines in view whose tasks could not be read (or have not come yet), each with why: what the list says
/// where their tasks would be. On that machine's own board the whole of it follows.
pub fn unread_lines(s: &State, width: usize) -> Vec<Inked> {
    let all = crate::board::update::remotes::in_view(s);
    let whole = s.scope != "*";
    let mut out = vec![];
    for m in all.into_iter().filter(|m| !m.ok) {
        let (line, detail): (String, Vec<String>) = match &m.why {
            Some(w) => { let mut d = vec![]; if w.b("said") { d.push("What came back:".to_string()); d.extend(w.s("said").lines().map(|l| format!("  {l}"))); d.push(String::new()); } d.extend(w.arr("detail").iter().map(text)); (format!("{}: its tasks cannot be read: {}.", m.machine.label, w.s("line")), d) }
            None => (format!("{}: asking for its tasks…", m.machine.label), vec![]),
        };
        let more = if !detail.is_empty() && !whole { format!(" More: {}.", key_name("scope")) } else { String::new() };
        out.extend(Inked::of(format!("{line}{more}"), if detail.is_empty() { Ink::Muted } else { Ink::Yellow }).wrap(width.saturating_sub(2).max(10)).into_iter().map(|l| Inked::plain(" ").add(l)));
        if whole && !detail.is_empty() { out.push(Inked::new()); for d in detail { out.extend(Inked::of(d, Ink::Soft).wrap(width.saturating_sub(4).max(10)).into_iter().map(|l| Inked::plain("   ").add(l))); } }
    }
    out
}
/// A heading's line. A machine's in bold accent, a rule after it to the edge; a repo's in bold plain text, as loud as the
/// titles under it; a status group's in its colour, a machine's automations' in theirs. Each has a caret in the marks'
/// column, `▾` open, `▸` folded (then with how many tasks and automations it holds: `7 tasks, 1 automation`; a machine
/// with nothing in view: `0 tasks`).
/// A repo's heading has its key beside its name, dimmed (`gtm-agent · AGT`): the first thing to go where room is short.
/// A machine's or a project's row (and `All machines`) is its coordinator's, drawn as a task's row (view/pilot.rs). The
/// top row says what it holds, open or folded: `4 machines · 420 tasks`, `9 projects · 30 tasks`.
fn heading(s: &State, t: &Theme, rows: &[Row], at: usize, width: usize) -> Inked {
    let r = &rows[at];
    let Row::Head { name, autos, machine, folded, folded_autos, id, .. } = r else { return Inked::new() };
    let top = *id == crate::board::update::top::top_id(s);
    let project = id.starts_with(&format!("{}repo\t", crate::board::update::heads::HEAD));
    let key = if s.by_repo || (project && s.archive_view) { crate::board::tags::heading_key(s, at) } else { String::new() };
    let named = if *autos { Inked::of(name.clone(), Ink::Auto) } else if *machine || top { Inked::of(name.clone(), MACHINE_INK).bold() } else if s.by_repo || project { Inked::of(crate::board::update::heads::repo_shown(s, id.split('\t').nth(1).unwrap_or(""), name), REPO_HEAD_INK) } else { Inked::of(name.clone(), group_ink(name)) };
    let mut named = if key.is_empty() { named } else { named.add(Inked::of(format!(" · {key}"), Ink::Muted)) };
    let caret = if id.is_empty() { Inked::plain("  ") } else { Inked::of(if r.shut() { "▸ " } else { "▾ " }, Ink::Muted) };
    // What a folded row holds: its tasks and automations; under Coordinators its projects' coordinators.
    let holds = if crate::board::update::shows::coords(s) { format!("{folded} project coordinator{}", if *folded == 1 { "" } else { "s" }) } else { crate::board::update::heads::holds(*folded, *folded_autos) };
    if top { named = named.add(Inked::of(format!("  {}", top_says(s)), Ink::Muted)); } else if r.shut() { named = named.add(Inked::of(format!("  {holds}"), Ink::Muted)); }
    super::pilot::placed(caret, named, super::pilot::shown(s, t, id), width)
}
/// What the top row holds: every machine's (`4 machines · 420 tasks`), or the one machine's (`9 projects · 30 tasks`).
fn top_says(s: &State) -> String {
    let n = |k: usize, what: &str| format!("{k} {what}{}", if k == 1 { "" } else { "s" });
    let tasks = s.show_n.first().copied().unwrap_or(0);
    if s.scope == "*" { return format!("{} · {}", n(crate::board::update::shows::hosts(s).len(), "machine"), n(tasks, "task")); }
    let items = if s.scope.is_empty() { &s.data.items } else { match s.remotes.get(&s.scope) { Some(m) => &m.items, None => return n(tasks, "task") } };
    let mut repos: Vec<&str> = items.iter().filter(|r| !r.b("archivedAt")).map(|r| (if r.b("rid") { r.s("rid") } else { r.s("id") }).split('/').next().unwrap_or("")).collect();
    repos.sort(); repos.dedup();
    format!("{} · {}", n(repos.len(), "project"), n(tasks, "task"))
}
/// What an empty list says: nothing matches what is typed, nothing archived, nothing yet.
fn none_lines(s: &State, width: usize) -> Vec<Inked> {
    let typed = (if s.mode == Mode::Find { s.value() } else { s.filter.clone() }).trim().to_string();
    let none_text = if !s.filter.is_empty() || s.mode == Mode::Find { if typed.is_empty() { "Type to filter.".to_string() } else { format!("Nothing matches \"{typed}\".") } } else if s.archive_view { "No archived tasks.".into() } else { "Nothing here yet. Type what needs doing.".into() };
    Inked::of(none_text, Ink::Muted).wrap(width.saturating_sub(2).max(8)).into_iter().map(|l| Inked::plain(" ").add(l)).collect()
}
struct L { text: Inked, sel: bool, head: bool, n: usize, rest: usize, id: String, auto: bool }
fn line(text: Inked) -> L { L { text, sel: false, head: false, n: 0, rest: 0, id: String::new(), auto: false } }
/// The list in `h` lines. `screen`: the whole board's width, which decides whether live rows have their third line.
pub fn list_lines(s: &mut State, t: &Theme, width: usize, h: usize, screen: usize) -> Vec<Inked> {
    let tall = tall_at(screen);
    // The room the list has decides how many done rows show: when it changes, they are folded again.
    if h != s.list_room || tall != s.list_tall || width != s.list_width || s.sel != s.fold_sel { s.list_room = h; s.list_tall = tall; s.list_width = width; s.rows = fold_rows(s); }
    let unread = unread_lines(s, width);
    let has_rows = !s.rows.is_empty();
    if !has_rows {
        let none = none_lines(s, width);
        let mut all = vec![Inked::new()];
        all.extend(unread.iter().cloned());
        if unread.is_empty() || s.scope == "*" { if !unread.is_empty() { all.push(Inked::new()); } all.extend(none); }
        s.unread_max = all.len().saturating_sub(h);
        s.unread_top = s.unread_top.min(s.unread_max);
        let top = s.unread_top;
        let mut out: Vec<Inked> = all.iter().skip(top).take(h).cloned().collect();
        let more = |x: String| Inked::plain(" ").add(Inked::of(crate::text::cut(&x, width.saturating_sub(1).max(1)), Ink::Muted));
        if top > 0 && !out.is_empty() { out[0] = more(format!("↑ more above ({})", key_name("up"))); }
        if top + h < all.len() && out.len() > 1 { let n = out.len() - 1; out[n] = more(format!("↓ more below ({})", key_name("down"))); }
        while out.len() < h { out.push(Inked::new()); }
        s.list_ids = vec![String::new(); h];
        return out;
    }
    s.unread_max = 0;
    let mut lines: Vec<L> = if unread.is_empty() { vec![] } else { std::iter::once(line(Inked::new())).chain(unread.into_iter().map(line)).collect() };
    // One layout for the displayed list, across groups: the columns line up down the whole list.
    let mut sections: Vec<Vec<Rec>> = vec![];
    let auto_rec = |a: &Obj| Rec::new(obj(json!({ "id": a.s("row"), "auto": a })));
    // A section is the rows of one heading of one kind: by repo, a repo's automations are a section after its tasks'.
    let mut kind = false;
    for r in &s.rows { let auto = matches!(r, Row::Auto(_)); if matches!(r, Row::Head { .. }) || sections.is_empty() || (auto != kind && matches!(r, Row::Rec(_) | Row::Auto(_))) { sections.push(vec![]); kind = auto; } match r { Row::Rec(o) => sections.last_mut().unwrap().push(o.clone()), Row::Auto(a) => sections.last_mut().unwrap().push(auto_rec(a)), _ => {} } }
    let flat: Vec<Rec> = sections.iter().flatten().cloned().collect();
    // Each row stands as far in as it is deep (update/depth.rs): the columns are laid out for the deepest.
    let rows = s.rows.clone();
    let ind = crate::board::update::depth::indents(&rows, width);
    let deep = crate::board::update::depth::depths(&rows);
    // An open heading holds what follows it: a row, or a heading deeper in. One that holds nothing (a project's row
    // under Coordinators) is a row of its own, never left hanging at the foot of what shows.
    let holds = |k: usize| match rows.get(k + 1) { Some(Row::Head { .. }) => deep[k + 1] > deep[k], Some(_) => true, None => false };
    let deepest = ind.iter().copied().max().unwrap_or(0);
    let lay = row_layout(s, t, &flat, width.saturating_sub(deepest), &sections, screen);
    let pad = |n: usize, l: Inked| if n == 0 { l } else { Inked::spaces(n).add(l) };
    for (k, r) in rows.iter().enumerate() {
        let (n_in, width) = (ind[k], width.saturating_sub(ind[k]));
        match r {
            Row::Head { id, .. } => {
                if heading_height(&rows, k) > 1 { lines.push(L { head: true, ..line(Inked::new()) }); }
                // An open heading at the foot of what shows goes with the empty line above it; a folded one is a row.
                let text = heading(s, t, &rows, k, width);
                let sel = !id.is_empty() && *id == s.sel;
                lines.push(L { head: !r.shut() && holds(k), sel, id: id.clone(), ..line(pad(n_in, text)) });
            }
            Row::Note(n) => lines.push(line(pad(n_in, Inked::plain("   ").add(Inked::of(n.clone(), Ink::Muted))))),
            Row::Rec(_) | Row::Auto(_) => {
                let rec: Rec = match r { Row::Auto(a) => auto_rec(a), Row::Rec(o) => o.clone(), _ => unreachable!() };
                let all = row_lines(s, t, &rec, width, &lay);
                let k = all.len();
                for (n, text) in all.into_iter().enumerate() { lines.push(L { text: pad(n_in, text), sel: rec.s("id") == s.sel, head: false, n, rest: k - n, id: rec.t("id"), auto: matches!(r, Row::Auto(_)) }); }
            }
        }
    }
    // Only the top row left (nothing yet, nothing that matches): what the list says stands under it.
    let top_alone = matches!(rows.as_slice(), [Row::Head { id, folded: 0, .. }] if *id == crate::board::update::top::top_id(s));
    if top_alone { lines.push(line(Inked::new())); lines.extend(none_lines(s, width).into_iter().map(line)); }
    // A list longer than its room keeps its last line to say how many tasks are not shown (not in a list of a few lines).
    let cut_any = lines.len() > h && h >= 8;
    let room = if cut_any { h - 1 } else { h };
    let at = lines.iter().position(|l| l.sel);
    let end = lines.iter().rposition(|l| l.sel);
    if !s.top_free { if let (Some(at), Some(end)) = (at, end) {
        if at < s.top + 2 { s.top = at.saturating_sub(2); }
        if end >= s.top + room { s.top = at.min(end + 1 - room); }
    } }
    s.top = s.top.min(lines.len().saturating_sub(room));
    // The list scrolls by whole rows: a row of two or three lines is never shown in part.
    if let Some(first) = lines.get(s.top) { if first.n > 0 && !first.sel { if end.map(|e| (e as i64) < s.top as i64 - first.n as i64 + room as i64).unwrap_or(true) { s.top -= first.n; } else { s.top += first.rest; } } }
    let mut shown: Vec<L> = lines.iter().skip(s.top).take(room).map(|l| L { text: l.text.clone(), sel: l.sel, head: l.head, n: l.n, rest: l.rest, id: l.id.clone(), auto: l.auto }).collect();
    if let Some(cut) = lines.get(s.top + room) { if shown.len() == room && cut.n > 0 { for n in 1..=cut.n.min(room) { let k = room - n; if !shown[k].sel { shown[k] = line(Inked::new()); } } } }
    for k in (0..shown.len()).rev() { if (shown[k].head && !shown[k].sel) || shown[k].text.is_empty() { shown[k] = line(Inked::new()); } else { break; } }
    s.list_ids = shown.iter().map(|l| l.id.clone()).collect();
    let mut out: Vec<Inked> = shown.iter().map(|l| if l.sel { hl(t, &l.text, width) } else { l.text.clone() }).collect();
    while out.len() < h { out.push(Inked::new()); }
    if cut_any {
        let ids = |auto: bool| { let mut v: Vec<String> = vec![]; for l in &lines { if !l.id.is_empty() && !is_head(&l.id) && l.auto == auto && !v.contains(&l.id) { v.push(l.id.clone()); } } v };
        let on = |list: &[String]| list.iter().filter(|id| s.list_ids.contains(id)).cloned().collect::<Vec<_>>();
        let (all, autos) = (ids(false), ids(true));
        let seen = on(&all);
        // Tasks under a folded heading are not counted: the heading says how many it holds.
        let shut: usize = s.rows.iter().map(|r| if let Row::Head { folded, folded_autos, .. } = r { folded - folded_autos } else { 0 }).sum();
        let total: usize = s.grouped.iter().map(|(_, r)| r.len()).sum::<usize>().saturating_sub(shut);
        // Above the first line shown, or below the last: rows that start there.
        let first = |id: &String| lines.iter().position(|l| l.id == *id).unwrap_or(0);
        let before = |list: &[String]| list.iter().filter(|id| !s.list_ids.contains(id) && first(id) < s.top).count();
        let above = before(&all);
        let below = total.saturating_sub(seen.len() + above);
        let autos_above = before(&autos);
        let autos_below = autos.len().saturating_sub(on(&autos).len() + autos_above);
        let said = |arrow: &str, n: usize, a: usize| if n > 0 || a > 0 { format!("{arrow} {}", [(n > 0).then(|| format!("{n} more")), (a > 0).then(|| format!("{a} automation{}", if a == 1 { "" } else { "s" }))].into_iter().flatten().collect::<Vec<_>>().join(", ")) } else { String::new() };
        let note = [said("↓", below, autos_below), said("↑", above, autos_above)].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
        out[h - 1] = if note.is_empty() { Inked::new() } else { Inked::plain(" ").add(Inked::of(crate::text::cut(&note, width.saturating_sub(1).max(1)), Ink::Muted)) };
    }
    out
}
