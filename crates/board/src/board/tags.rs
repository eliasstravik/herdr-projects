//! A task's number as the board says it: the quiet tag of its row, `#134`, or `HSW-134` where its repo has a short key
//! (store/keys.rs). A row's tag never names its machine, which the heading above it does (update/heads.rs); a message
//! with every machine shown does (`otherbox#1`). Also how a message names a task: its tag and its title. On an automation's row the word on or off stands where a task has its number.
use crate::json::*;
use crate::store::keys::Keys;
use crate::text::{cut, width};

/// A tag is cut to this many cells (a key has 5 at most, so only another machine's could be longer); one that names its
/// machine too to FAR_MAX.
const TAG_MAX: usize = 12;
const FAR_MAX: usize = 14;
/// The tag gives way, on every row at once, where it would leave a row's name fewer cells than this.
pub const NAME_LEFT: usize = 16;
fn id_of(r: &Obj) -> String { if r.b("rid") { r.t("rid") } else { r.t("id") } }
fn clean(s: &str) -> String { crate::shown::words::clean(s) }
/// The tag in words. `scope`: whose board shows ("*" every machine: another machine's tag names its machine). `machine`:
/// false in find, which takes the number alone.
pub fn number_tag(r: &Obj, keys: &Keys, scope: &str, machine: bool) -> String {
    if r.has("auto") { return if r.at("auto").b("on") { "on".into() } else { "off".into() }; }
    let id = id_of(r);
    let num = id.rsplit('/').next().unwrap_or("").to_string();
    let repo = id.split('/').next().unwrap_or("").to_string();
    if !r.has("title") || num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) { return String::new(); }
    let key = if r.b("host") { r.t("key") } else { keys.get(&repo).cloned().unwrap_or_default() };
    let far = if machine && r.b("host") && scope == "*" { clean(r.s("host")) } else { String::new() };
    let own = cut(&if key.is_empty() { format!("#{num}") } else { format!("{}-{num}", clean(&key)) }, TAG_MAX);
    if far.is_empty() { return own; }
    let sep = if key.is_empty() { "" } else { ":" };
    format!("{}{sep}{own}", cut(&far, 3.max(FAR_MAX.saturating_sub(width(&own) + sep.len()))))
}
/// The tag on a row of the list: never its machine, which the heading above it names.
pub fn row_tag(r: &Obj, keys: &Keys) -> String { number_tag(r, keys, "", false) }
/// The column for tags of these widths: as wide as the widest, so every title in view starts at the same place
/// (`HP-1   ` lines up with `HSW-206`).
pub fn tag_column(widths: &[usize]) -> usize { widths.iter().copied().max().unwrap_or(0) }
/// The cells a tag this wide fills in that column, without the cell that parts it from its title.
pub fn tag_cells(w: usize, col: usize) -> usize { if col == 0 || w == 0 { 0 } else { w.max(col) } }
/// What the tags of a list are made from: the column, and the widest tag.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Numbering { pub cells: usize, pub widest: usize }
pub fn numbering(recs: &[&Obj], keys: &Keys, autos: bool) -> Numbering {
    let widths: Vec<usize> = recs.iter().map(|r| width(&row_tag(r, keys))).filter(|w| *w > 0).collect();
    let mut n = Numbering { cells: tag_column(&widths), widest: widths.iter().copied().max().unwrap_or(0) };
    // An automation's row has the word on or off where a task has its number: the column is there for it too.
    if autos { n.cells = n.cells.max(3); n.widest = n.widest.max(3); }
    n
}
/// The tags' column in a list this wide: none where the tags gave way.
pub fn number_cells(width: usize, n: Numbering) -> usize { if n.cells > 0 && width >= 3 + n.widest + 1 + NAME_LEFT { n.cells } else { 0 } }
/// The key beside a repo's name where the board names the repo (`gtm-agent · AGT`), never on a task's row, which starts
/// with it. The heading's id names whose repo it is: this machine's has this machine's key; another machine's the one
/// its tasks came with (folded or not), and none where it has only automations here (that machine sends no key, and
/// this machine's are not its). "" for a heading that is no repo's.
pub fn heading_key(s: &crate::board::state::State, at: usize) -> String {
    use crate::board::state::Row;
    let Some(Row::Head { name, id, machine: false, autos: false, .. }) = s.rows.get(at) else { return String::new() };
    let host = id.split('\t').nth(1).unwrap_or("");
    if host.is_empty() { return s.data.keys.get(name).cloned().unwrap_or_default(); }
    s.items.iter().find(|r| r.s("host") == host && r.s("rid").split('/').next() == Some(name.as_str())).map(|r| r.t("key")).unwrap_or_default()
}
/// A repo named with its key: `gtm-agent · AGT`, or the name alone where it has none.
pub fn repo_with_key(name: &str, key: &str) -> String { if key.is_empty() { name.to_string() } else { format!("{name} · {key}") } }
/// A task as a message names it: `#134 "Fix the footer"`, with its machine where you look at more than one.
pub fn name_say(r: &Obj, keys: &Keys, scope: &str, max: usize) -> String {
    let tag = number_tag(r, keys, scope, true);
    let title = clean(r.s("title"));
    let mut parts = vec![];
    if !tag.is_empty() { parts.push(tag); } else if title.is_empty() { parts.push(r.t("id")); }
    if !title.is_empty() { parts.push(format!("\"{}\"", if max == usize::MAX { title } else { cut(&title, max) })); }
    if r.b("host") && scope != "*" { parts.push(format!("on {}", clean(r.s("host")))); }
    parts.join(" ")
}
pub fn id_say(r: &Obj, keys: &Keys, scope: &str) -> String { let t = number_tag(r, keys, scope, true); if t.is_empty() { r.t("id") } else { t } }
/// A task's id as the panel says it: with a key, the short name and the id it stands for.
pub fn id_words(r: &Obj, keys: &Keys, scope: &str) -> String {
    let said = id_say(r, keys, scope);
    let label = crate::shown::words::id_label(r.s("id"));
    if said == r.s("id") { label } else { format!("{said} · {label}") }
}
