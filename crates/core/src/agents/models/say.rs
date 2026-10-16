//! `herdr-projects models [program]`: the model names each program takes, as a setting writes them (program/model),
//! and the effort levels it takes.
use super::super::efforts::{levels, levels_by, Ask};
use super::super::kinds::{all, kind, kind_named, KIND_NAMES};
use super::{how, names_of, Listing};
use crate::err::Res;
use crate::json::*;

/// One program in a line; listed names, up to this many.
const IN_LINE: usize = 4;
/// The effort levels it takes (efforts.rs), in a few words, and as data: its levels, and each model's own where they
/// differ (Codex).
fn efforts_said(k: &str) -> (String, Value) {
    let Some(l) = levels(k, "", Ask::Fresh) else {
        let note = kind(k).map(|a| a.effort_note).filter(|n| !n.is_empty()).map(String::from).unwrap_or_else(|| format!("{k} takes no effort level when the board starts it"));
        return (note, json!({ "levels": null }));
    };
    if l.levels.is_empty() { return ("any level".into(), json!({ "levels": [] })); }
    let mut by = Obj::new();
    let mut odd: Vec<(Vec<String>, Vec<String>)> = vec![];
    for (m, own) in levels_by(k) {
        if own == l.levels { continue; }
        by.insert(m.clone(), json!(own));
        match odd.iter_mut().find(|(o, _)| *o == own) { Some((_, ms)) => ms.push(m), None => odd.push((own, vec![m])) }
    }
    let from = if l.from.is_empty() { String::new() } else { format!(" (from `{}`)", l.from) };
    let odd: String = odd.iter().map(|(o, ms)| format!("; {} only {}", ms.join(", "), o.join(", "))).collect();
    (format!("{}{from}{odd}", l.levels.join(", ")), json!({ "levels": l.levels, "by": by }))
}
fn shown_of(l: Listing) -> Option<Vec<String>> { match l { Listing::Names(n) => Some(if n.shown.is_empty() { n.taken } else { n.shown }), _ => None } }
/// The sign-in its names are for, where it said one.
fn signin_of(l: &Listing) -> String { match l { Listing::Names(n) => n.signin.clone(), _ => String::new() } }
pub fn models_command(only: &str) -> Res<(Value, String)> {
    if !only.is_empty() {
        let Some(k) = kind_named(only.split('/').next().unwrap_or("")) else {
            let near = crate::store::settings::values::nearest(only, &KIND_NAMES).map(|n| format!(" Did you mean {n}?")).unwrap_or_default();
            crate::refuse!("No agent program \"{only}\". It must be {}.{near}", KIND_NAMES.join(", "));
        };
        let k = k.name;
        let names = names_of(k, true);
        if let Listing::Failed(why) = &names { crate::refuse!("{k}'s list of models could not be read: {why}."); }
        let signin = signin_of(&names);
        let shown = shown_of(names);
        let (effort, efforts) = efforts_said(k);
        let here = if signin.is_empty() { String::new() } else { format!(" (here: your {signin})") };
        let text = match &shown {
            Some(s) => format!("{k} takes {}{here}. Write one after the program, as in worker.agent {k}/{}:\n\n{}", how(k), s[0], s.iter().map(|m| format!("{k}/{m}")).collect::<Vec<_>>().join("\n")),
            None => format!("{k}: {}.", how(k)),
        };
        let text = format!("{text}\n\nEffort (worker.effort and the like): {effort}.");
        return Ok((json!({ "agent": k, "how": how(k), "models": shown, "signin": signin, "efforts": efforts }), text));
    }
    let mut rows = vec![];
    let mut lines = vec!["A model goes after its program, as in worker.agent codex/<model>; without one the program uses its own default. A name or an effort it does not take is refused with the ones it does.".to_string(), String::new()];
    for a in all() {
        let k = a.name;
        let names = names_of(k, false);
        let failed = if let Listing::Failed(why) = &names { Some(why.clone()) } else { None };
        let signin = signin_of(&names);
        let shown = shown_of(names);
        let line = match &shown {
            _ if failed.is_some() => format!("its list could not be read: {}", failed.clone().unwrap_or_default()),
            Some(s) if s.len() > IN_LINE => format!("{} names, such as {}: herdr-projects models {k} lists them all", s.len(), s.iter().take(IN_LINE).map(|m| format!("{k}/{m}")).collect::<Vec<_>>().join(", ")),
            Some(s) => s.iter().map(|m| format!("{k}/{m}")).collect::<Vec<_>>().join(", "),
            None => how(k),
        };
        let (effort, efforts) = efforts_said(k);
        lines.push(format!("{k:<11} {line}{}", if signin.is_empty() { String::new() } else { format!(" (for your {signin})") }));
        lines.push(format!("{:<11} effort: {effort}", ""));
        rows.push(json!({ "agent": k, "how": how(k), "models": shown, "signin": signin, "efforts": efforts }));
    }
    Ok((Value::Array(rows), lines.join("\n")))
}
