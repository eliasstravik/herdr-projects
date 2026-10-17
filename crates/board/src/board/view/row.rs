//! One row of the list: its mark, its number, its name, its columns on the right and, for a live task, a second line
//! that says what it does (a third on a narrow screen: row_tall.rs, where long ones wrap: row_wrap.rs). The columns
//! are laid out once for the whole list, so each starts at the same place on every row; what is shown of them is
//! decided one section at a time (row_fit.rs). One cell between two columns, and one wider gap of two cells on a row:
//! right of the progress (bar, percent, minutes left), before the review, the pull request and the time.
use super::fields::{fields, is_auto, mark, name_ink, row_title, says_beside, says_under, F};
use super::row_fit::{label_room, shown_from, Cell, Fit, ETA_STEADY, PERCENT_MIN, PERCENT_STEADY};
use super::row_tall::{tall_at, tall_layout, Third};
use super::row_wrap::{wrapped_lines, wraps};
use crate::board::state::State;
use crate::board::tags::{number_cells, numbering, row_tag, tag_cells, Numbering};
use crate::board::theme::Theme;
use crate::json::*;
use crate::shown::ink::{Ink, Inked};
use crate::shown::states::is_live;
use crate::text::width;
use std::collections::HashMap;

/// The fewest cells a cut "doing now" sentence is still worth showing in.
const SENTENCE_MIN: usize = 20;
const PROGRESS: [&str; 3] = ["bar", "percent", "eta"];

#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub now: i64, pub pr: usize, pub elapsed: usize, pub cols: Vec<(&'static str, usize)>, pub auto_cols: Vec<(&'static str, usize)>,
    pub num: usize, pub numbering: Numbering, pub ago: bool, pub show: HashMap<String, (usize, bool, usize)>, pub tall: bool, pub third: Option<Third>, pub repo_col: bool,
}
fn say_need(said: &Inked, r: &Obj) -> usize { if is_auto(r) { said.width() } else { said.width().min(SENTENCE_MIN) } }
pub fn number_wide(s: &State, r: &Obj, cells: usize) -> usize {
    if is_auto(r) { return if cells > 0 { cells + 1 } else { 0 }; }
    let w = if cells > 0 { tag_cells(width(&row_tag(r, &s.data.keys)), cells) } else { 0 };
    if w > 0 { w + 1 } else if cells > 0 { cells + 1 } else { 0 }
}
/// The tag as a row starts with it: greyed, filled out to the column, one cell before the name.
pub fn number_lead(s: &State, r: &Obj, cells: usize) -> Inked {
    if cells == 0 { return Inked::new(); }
    let tag = row_tag(r, &s.data.keys);
    let n = number_wide(s, r, cells);
    Inked::of(tag.clone(), Ink::Muted).add(Inked::spaces(n.saturating_sub(width(&tag))))
}
/// The numbering of the list as it is (or of `recs` where there is no list).
pub fn list_numbering(s: &State, recs: &[&Obj]) -> Numbering {
    let all: Vec<&Obj> = s.grouped.iter().flat_map(|(_, l)| l.iter().map(|r| &**r)).collect();
    let src: Vec<&Obj> = if all.is_empty() { recs.to_vec() } else { all };
    numbering(&src, &s.data.keys, !s.autos.is_empty())
}
/// The columns, once for the whole list, and what each section of it shows.
pub fn row_layout(s: &mut State, t: &Theme, records: &[crate::board::state::Rec], width_: usize, sections: &[Vec<crate::board::state::Rec>], screen: usize) -> Layout {
    let now = s.now;
    let tall = tall_at(screen);
    let bare = |r: &Obj, s: &State| wraps(s, r, tall);
    let fields_of: Vec<(usize, F)> = records.iter().enumerate().filter(|(_, r)| !bare(r, s)).map(|(n, r)| (n, fields(s, t, r, now))).collect();
    let fmap: HashMap<usize, &F> = fields_of.iter().map(|(n, f)| (*n, f)).collect();
    let max = |k: &str| fields_of.iter().map(|(_, f)| if k == "elapsed" { width(&f.elapsed) } else { f.get(k).width() }).max().unwrap_or(0);
    let elapsed = max("elapsed").max(3);
    let recs_ref: Vec<&Obj> = records.iter().map(|r| &**r).collect();
    let nb = list_numbering(s, &recs_ref);
    let num = number_cells(width_, nb);
    s.tags_gone = num == 0;
    // Extremely long PR identifiers may shorten, but never push the time off-screen.
    let pr = max("pr").min(width_.saturating_sub(3 + elapsed + 3));
    let pct = if max("percent") > 0 { max("percent").max(PERCENT_MIN) } else { 0 };
    let mut cols: Vec<(&'static str, usize)> = [("bar", max("bar")), ("percent", pct), ("eta", max("eta")), ("review", max("review")), ("pr", pr), ("time", if tall { 0 } else { elapsed })].into_iter().filter(|(_, w)| *w > 0).collect();
    // The wider gap is a column with nothing in it, right of the progress.
    if let Some(g) = cols.iter().position(|(k, _)| !PROGRESS.contains(k)) { if g > 0 { cols.insert(g, ("gap", 0)); } }
    let own = |k: &str| records.iter().enumerate().filter(|(_, r)| is_auto(r)).map(|(n, _)| fmap.get(&n).map(|f| f.get(k).width()).unwrap_or(0)).max().unwrap_or(0);
    let auto_cols: Vec<(&'static str, usize)> = [("tag", own("tag")), ("lead", own("lead")), ("time", if tall { 0 } else { elapsed })].into_iter().filter(|(_, w)| *w > 0).collect();
    let mut lay = Layout { now, pr, elapsed, cols: cols.clone(), auto_cols: auto_cols.clone(), num, numbering: nb, ago: !tall && fields_of.iter().any(|(_, f)| f.ago), show: HashMap::new(), tall, third: None, repo_col: crate::board::view::repo_col::wanted(s) };
    if tall {
        let live: Vec<F> = records.iter().filter(|r| tall && is_live(r)).map(|r| fields(s, t, r, now)).collect();
        lay.third = Some(tall_layout(&live, width_, 3 + if num > 0 { num + 1 } else { 0 }));
    }
    let cells = |ago: bool, cols: &[(&'static str, usize)]| -> Vec<Cell> { cols.iter().map(|(k, w)| Cell { key: k, w: match *k { "percent" => (*w).max(PERCENT_STEADY), "eta" => (*w).max(ETA_STEADY), "time" if ago => w + 4, _ => *w } }).collect() };
    let cols_of = |recs: &[crate::board::state::Rec]| if recs.first().map(|r| is_auto(r)).unwrap_or(false) { auto_cols.clone() } else { cols.clone() };
    let fit = |r: &Obj, f: Option<&F>, s: &State| Fit {
        room: width_.saturating_sub(mark(t, r).width() + 2 + number_wide(s, r, num)), name: width(&row_title(s, r)),
        has: ["bar", "percent", "eta", "review", "pr", "tag", "lead"].into_iter().filter(|k| f.map(|f| f.has(k)).unwrap_or(false)).chain(if tall { None } else { Some("time") }).collect(),
    };
    let field_for = |r: &Obj| records.iter().position(|x| x.s("id") == r.s("id")).and_then(|n| fmap.get(&n).copied());
    let parts: Vec<Vec<Fit>> = sections.iter().map(|recs| recs.iter().map(|r| fit(r, field_for(r), s)).collect()).collect();
    // The word "ago" sits right of everything: the whole list gives it up as soon as that lets some section show more.
    if lay.ago {
        let cheaper = |a: (usize, usize), b: (usize, usize)| a.0 < b.0 || (a.0 == b.0 && a.1 < b.1);
        lay.ago = !parts.iter().enumerate().any(|(n, rows)| cheaper(shown_from(rows, &cells(false, &cols_of(&sections[n]))).1, shown_from(rows, &cells(true, &cols_of(&sections[n]))).1));
    }
    for (n, recs) in sections.iter().enumerate() {
        let steady = cells(lay.ago, &cols_of(recs));
        let (from, _) = shown_from(&parts[n], &steady);
        let rooms: Vec<usize> = recs.iter().enumerate().map(|(m, _)| label_room(&parts[n][m], &steady, from).max(0) as usize).collect();
        // What the rows say goes the same way: every row of the section that says something has room for it, or none says it.
        let says = recs.iter().enumerate().all(|(m, r)| { let said = says_beside(s, r, tall); said.is_empty() || rooms[m] as i64 - row_name(s, r, rooms[m]).width() as i64 - 2 >= say_need(&said, r) as i64 });
        for (m, r) in recs.iter().enumerate() { lay.show.insert(r.t("id"), (from, says, rooms[m])); }
    }
    lay
}
pub fn row_name(s: &State, r: &Obj, w: usize) -> Inked { name_ink(r, &row_title(s, r)).cut(w) }
fn fill(x: &Inked, n: usize, left: bool) -> Inked { let gap = Inked::spaces(n.saturating_sub(x.width())); if left { x.clone().add(gap) } else { gap.add(x.clone()) } }
/// The columns one row shows, drawn: its own from where its section's start.
pub fn row_keeps(s: &State, t: &Theme, r: &Obj, lay: &Layout) -> Inked {
    if wraps(s, r, lay.tall) { return Inked::new(); }
    let f = fields(s, t, r, lay.now);
    let time = Inked::of(format!("{}{}", crate::text::pad_left(&f.elapsed, lay.elapsed), if lay.ago { if f.ago { " ago" } else { "    " } } else { "" }), Ink::Muted);
    let text = |k: &str| -> Inked { match k { "pr" => if f.pr.is_empty() { Inked::new() } else { fill(&f.pr.cut(lay.pr), lay.pr, true) }, "time" => time.clone(), "gap" => Inked::new(), k => f.get(k) } };
    let cols = if is_auto(r) { &lay.auto_cols } else { &lay.cols };
    let mut cells: Vec<(&str, Inked, usize)> = cols.iter().map(|(k, w)| (*k, text(k), w + if *k == "time" && lay.ago { 4 } else { 0 })).collect();
    // An automation that never fired has no time: "never fired" stands where the others have their word and their time.
    if f.never && cells.len() >= 2 && cells[cells.len() - 2].0 == "lead" {
        let w = cells[cells.len() - 2].2 + 1 + cells[cells.len() - 1].2;
        let word = Inked::of(if w >= 11 { "never fired" } else { "never" }, Ink::Muted);
        cells.truncate(cells.len() - 2);
        cells.push(("time", word, w));
    }
    // Blank cells left of a row's first own column are not columns to that row: its words run into them.
    let mut from = lay.show.get(r.s("id")).map(|x| x.0).unwrap_or(0);
    while from < cells.len() && cells[from].1.is_empty() { from += 1; }
    let mut out = Inked::new();
    for (n, (k, x, w)) in cells[from.min(cells.len())..].iter().enumerate() { if n > 0 { out.push(" ", Ink::Plain); } out = out.add(fill(x, *w, *k == "pr")); }
    out
}
/// A row's first line, measured: its mark and number, the columns it keeps, and the room they leave its name.
pub struct Room { pub prefix: Inked, pub room: usize, pub metadata: Inked, pub label_room: usize, pub says: bool }
pub fn row_room(s: &State, t: &Theme, r: &Obj, w: usize, lay: &Layout) -> Room {
    let prefix = Inked::plain(" ").add(mark(t, r)).add(Inked::plain(" ")).add(number_lead(s, r, lay.num));
    let room = w.saturating_sub(prefix.width());
    let metadata = row_keeps(s, t, r, lay);
    let show = lay.show.get(r.s("id")).copied();
    let beside = room.saturating_sub(metadata.width() + if metadata.is_empty() { 0 } else { 2 });
    Room { label_room: show.map(|x| x.2).unwrap_or(room).min(beside), says: show.map(|x| x.1).unwrap_or(true), prefix, room, metadata }
}
/// One rule for every row: the columns first, the name in what they leave, what the row says in what is then left.
pub fn row(s: &State, t: &Theme, r: &Obj, w: usize, lay: &Layout) -> Inked {
    let rm = row_room(s, t, r, w, lay);
    let said = if rm.says { says_beside(s, r, lay.tall) } else { Inked::new() };
    let title = row_name(s, r, rm.label_room);
    let say_room = rm.label_room as i64 - title.width() as i64 - 2;
    // A sentence cut to a few letters says nothing: it shows whole, or with at least SENTENCE_MIN cells.
    let tail = if said.is_empty() || say_room < say_need(&said, r) as i64 { Inked::new() } else { said.cut(say_room as usize) };
    let label = if tail.is_empty() { title } else { title.add(Inked::plain("  ")).add(tail) };
    let gap = rm.room.saturating_sub(label.width() + rm.metadata.width());
    // The repo's name takes only what the title, the columns and what the row says leave over.
    let (repo, gap) = super::repo_col::beside(r, lay, gap);
    rm.prefix.add(label).add(Inked::spaces(gap)).add(repo).add(rm.metadata)
}
/// A live task's second line: what it says, under its name, with the row's whole width; cut at its end.
pub fn status_line(s: &State, t: &Theme, r: &Obj, w: usize, lay: &Layout) -> Inked {
    let rm = row_room(s, t, r, w, lay);
    let lead = Inked::spaces(rm.prefix.width());
    lead.add(says_under(s, r, rm.says)).cut(w)
}
/// A row as the list draws it: one line, two for a live task (an automation that says something under its name).
pub fn row_lines(s: &State, t: &Theme, r: &Obj, w: usize, lay: &Layout) -> Vec<Inked> {
    if wraps(s, r, lay.tall) { return wrapped_lines(s, t, r, w, lay); }
    let two = if is_auto(r) { !says_under(s, r, lay.show.get(r.s("id")).map(|x| x.1).unwrap_or(true)).is_empty() } else { is_live(r) };
    if two { vec![row(s, t, r, w, lay), status_line(s, t, r, w, lay)] } else { vec![row(s, t, r, w, lay)] }
}
