//! The filter row for the web board, as the terminal board has it under its header (board/view/shows_row.rs): whose
//! board first, where there is another machine (`All machines`, or the one machine shown, lit), then every filter with
//! its count (All, Coordinators, Needs you, Working, Ready, To do, Done, Automations), the one shown lit in its own
//! colour, a count of what needs you in red. None in the archive, which has no filter row. A tap on a filter shows it
//! (web.rs: `show`); a tap on the machine picks whose board.
use crate::board::state::State;
use crate::board::update::shows::{NEEDS, SHOWS};
use crate::board::view::shows_row::{INKS, SHORT};
use crate::json::*;

pub fn shows_of(s: &State) -> Value {
    if s.archive_view { return Value::Null; }
    let machine = (!s.remotes.is_empty() || s.all()).then(|| {
        let name = if s.scope == "*" { "All machines".to_string() } else { crate::board::update::remotes::shown(s, &s.scope) };
        let ok = s.scope.is_empty() || s.scope == "*" || s.remotes.get(&s.scope).is_some_and(|m| m.ok);
        json!({ "name": name, "lit": s.scope != "*", "ok": ok })
    });
    let items: Vec<Value> = SHOWS.iter().enumerate().map(|(n, (name, _))| {
        let count = s.show_n.get(n).copied().unwrap_or(0);
        json!({ "n": n, "name": name, "short": SHORT[n], "count": count, "lit": n == s.show, "color": INKS[n].name(), "red": n == NEEDS && count > 0 })
    }).collect();
    json!({ "machine": machine, "items": items })
}
