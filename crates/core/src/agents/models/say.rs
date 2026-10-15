//! `herdr-projects models [program]`: the model names each program takes, as a setting writes them (program/model).
use super::super::kinds::{all, kind_named, KIND_NAMES};
use super::{how, listed};
use crate::err::Res;
use crate::json::*;

/// One program in a line; listed names, up to this many.
const IN_LINE: usize = 4;
pub fn models_command(only: &str) -> Res<(Value, String)> {
    if !only.is_empty() {
        let Some(k) = kind_named(only.split('/').next().unwrap_or("")) else {
            let near = crate::store::settings::values::nearest(only, &KIND_NAMES).map(|n| format!(" Did you mean {n}?")).unwrap_or_default();
            crate::refuse!("No agent program \"{only}\". It must be {}.{near}", KIND_NAMES.join(", "));
        };
        let k = k.name;
        let names = listed(k, true);
        let shown = names.as_ref().map(|n| if n.shown.is_empty() { n.taken.clone() } else { n.shown.clone() });
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
        let shown = listed(k, false).map(|n| if n.shown.is_empty() { n.taken } else { n.shown });
        let line = match &shown {
            Some(s) if s.len() > IN_LINE => format!("{} names, such as {}: herdr-projects models {k} lists them all", s.len(), s.iter().take(IN_LINE).map(|m| format!("{k}/{m}")).collect::<Vec<_>>().join(", ")),
            Some(s) => s.iter().map(|m| format!("{k}/{m}")).collect::<Vec<_>>().join(", "),
            None => how(k),
        };
        lines.push(format!("{k:<11} {line}"));
        rows.push(json!({ "agent": k, "how": how(k), "models": shown }));
    }
    Ok((Value::Array(rows), lines.join("\n")))
}
