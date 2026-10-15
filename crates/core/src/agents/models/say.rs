//! `herdr-projects models [program]`: the model names each program takes, as a setting writes them (program/model).
use super::super::kinds::{all, kind_named, KIND_NAMES};
use super::{how, names_of, Listing};
use crate::err::Res;
use crate::json::*;

/// One program in a line; listed names, up to this many.
const IN_LINE: usize = 4;
fn shown_of(l: Listing) -> Option<Vec<String>> { match l { Listing::Names(n) => Some(if n.shown.is_empty() { n.taken } else { n.shown }), _ => None } }
pub fn models_command(only: &str) -> Res<(Value, String)> {
    if !only.is_empty() {
        let Some(k) = kind_named(only.split('/').next().unwrap_or("")) else {
            let near = crate::store::settings::values::nearest(only, &KIND_NAMES).map(|n| format!(" Did you mean {n}?")).unwrap_or_default();
            crate::refuse!("No agent program \"{only}\". It must be {}.{near}", KIND_NAMES.join(", "));
        };
        let k = k.name;
        let names = names_of(k, true);
        if let Listing::Failed(why) = &names { crate::refuse!("{k}'s list of models could not be read: {why}."); }
        let shown = shown_of(names);
        let text = match &shown {
            Some(s) => format!("{k} takes {}. Write one after the program, as in worker.agent {k}/{}:\n\n{}", how(k), s[0], s.iter().map(|m| format!("{k}/{m}")).collect::<Vec<_>>().join("\n")),
            None => format!("{k}: {}.", how(k)),
        };
        return Ok((json!({ "agent": k, "how": how(k), "models": shown }), text));
    }
    let mut rows = vec![];
    let mut lines = vec!["A model goes after its program, as in worker.agent codex/<model>; without one the program uses its own default. A name it does not take is refused with the ones it does.".to_string(), String::new()];
    for a in all() {
        let k = a.name;
        let names = names_of(k, false);
        let failed = if let Listing::Failed(why) = &names { Some(why.clone()) } else { None };
        let shown = shown_of(names);
        let line = match &shown {
            _ if failed.is_some() => format!("its list could not be read: {}", failed.clone().unwrap_or_default()),
            Some(s) if s.len() > IN_LINE => format!("{} names, such as {}: herdr-projects models {k} lists them all", s.len(), s.iter().take(IN_LINE).map(|m| format!("{k}/{m}")).collect::<Vec<_>>().join(", ")),
            Some(s) => s.iter().map(|m| format!("{k}/{m}")).collect::<Vec<_>>().join(", "),
            None => how(k),
        };
        lines.push(format!("{k:<11} {line}"));
        rows.push(json!({ "agent": k, "how": how(k), "models": shown }));
    }
    Ok((Value::Array(rows), lines.join("\n")))
}
