//! What a setting's value may be: a typed or hand-written value is checked against its entry in settings_list.rs and
//! comes back as the board keeps it, or is refused with the nearest right one as a hint.
use crate::agents::kinds::{kind, kind_named, KIND_NAMES};
use crate::agents::roles::refusal;
use crate::err::{Error, Res};
use crate::json::*;
use crate::store::attempts::{MAX_ATTEMPTS, TOO_MANY};
use crate::store::settings_list::{entry_of, names, Entry, Ty};

pub const NONE: &str = "none";
fn refuse<T>(msg: String) -> Res<T> { Err(Error::refused(msg)) }

/// How many edits make one word another.
pub fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut d: Vec<Vec<usize>> = (0..=a.len()).map(|i| { let mut r = vec![0; b.len() + 1]; r[0] = i; r }).collect();
    for j in 1..=b.len() { d[0][j] = j; }
    for i in 1..=a.len() { for j in 1..=b.len() { d[i][j] = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + usize::from(a[i - 1] != b[j - 1])); } }
    d[a.len()][b.len()]
}
/// The nearest of a list of words to what was typed, when one is near enough to be meant.
pub fn nearest(word: &str, list: &[&str]) -> Option<String> {
    let w = word.to_lowercase();
    let tail = |x: &str| x.rsplit('.').next().unwrap_or("").to_lowercase();
    let mut best: Option<&str> = None;
    let mut score = usize::MAX;
    for x in list {
        let s = distance(&w, &x.to_lowercase()).min(distance(&tail(&w), &tail(x)) + if w.contains('.') { 1 } else { 2 });
        if s < score { best = Some(x); score = s; }
    }
    best.filter(|_| score <= 3.max(w.chars().count().div_ceil(2))).map(String::from)
}
fn hint(word: &str, list: &[&str]) -> String { nearest(word, list).map(|n| format!(" Did you mean {n}?")).unwrap_or_default() }
/// The entry of a name, or a refusal that names the nearest one.
pub fn known(name: &str) -> Res<&'static Entry> {
    match entry_of(name) { Some(e) => Ok(e), None => refuse(format!("Unknown setting \"{name}\".{} `herdr-projects show` lists them all.", hint(name, &names()))) }
}

// ---- a time carries a unit ---------------------------------------------------------------------------------------------
/// `90s`, `15m`, `2h` in milliseconds; 0 for anything else.
pub fn ms(time: &str) -> i64 {
    let t = time.trim();
    if t.len() < 2 { return 0; }
    let (n, u) = t.split_at(t.len() - 1);
    if !n.bytes().all(|b| b.is_ascii_digit()) { return 0; }
    let n: i64 = n.parse().unwrap_or(0);
    match u { "s" => n * 1000, "m" => n * 60_000, "h" => n * 3_600_000, _ => 0 }
}
fn none_meant(w: &str) -> String {
    let yes = ["no", "non", "off", "never", "unlimited", "nolimit"].contains(&w) || (w.starts_with('0') && w[1..].chars().all(|c| c.is_ascii_lowercase()));
    if yes { " Did you mean none?".into() } else { String::new() }
}
fn time_meant(w: &str, unit: &str) -> String {
    let digits: String = w.chars().take_while(|c| c.is_ascii_digit()).collect();
    let rest = &w[digits.len()..];
    if digits.is_empty() || !rest.chars().all(|c| c.is_ascii_lowercase()) || digits.parse::<u64>().unwrap_or(0) == 0 { return String::new(); }
    let u = match rest.chars().next() { None => unit.to_string(), Some('h') => "h".into(), Some('m') => "m".into(), Some('s') => "s".into(), Some(_) => unit.to_string() };
    format!("{digits}{u}")
}
// ---- a count of tokens: 500k, 2m, 1.5m -----------------------------------------------------------------------------------
fn big(u: &str) -> f64 { match u { "k" => 1e3, "m" => 1e6, "b" => 1e9, _ => 1.0 } }
fn split_count(s: &str) -> Option<(f64, String)> {
    let num: String = s.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    if num.is_empty() || num.starts_with('.') || num.ends_with('.') || num.matches('.').count() > 1 { return None; }
    Some((num.parse().ok()?, s[num.len()..].to_string()))
}
pub fn tokens_of(count: &str) -> i64 {
    match split_count(count) { Some((n, u)) if u.is_empty() || ["k", "m", "b"].contains(&u.as_str()) => (n * big(&u)).round() as i64, _ => 0 }
}
/// A count as the board keeps and says it: 2000000 is 2m, 1500000 is 1.5m, 500000 is 500k.
pub fn tokens_say(n: f64) -> String {
    let x = n.round().max(0.0);
    let cut = |v: f64| { let p = if v < 10.0 { 2 } else if v < 100.0 { 1 } else { 0 }; let s = format!("{v:.p$}"); let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }; s };
    if x >= 1e6 { format!("{}m", cut(x / 1e6)) } else if x >= 1e3 { format!("{}k", cut(x / 1e3)) } else { format!("{}", x as i64) }
}
fn tokens_meant(w: &str) -> String {
    let mut s: String = w.chars().filter(|c| *c != '_' && !c.is_whitespace()).collect();
    // A comma before three digits is a thousands mark; another is a decimal point.
    loop { let Some(i) = s.find(',') else { break }; let after: String = s[i + 1..].chars().take(3).collect(); if after.len() == 3 && after.chars().all(|c| c.is_ascii_digit()) { s.remove(i); } else { s.replace_range(i..i + 1, "."); } }
    let Some((n, u)) = split_count(&s) else { return String::new() };
    if !u.chars().all(|c| c.is_ascii_lowercase()) { return String::new(); }
    let unit = match u.chars().next() { Some('t') | Some('k') => "k", Some('m') => "m", Some('b') => "b", _ => "" };
    let n = n * big(unit);
    if n >= 1.0 { tokens_say(n) } else { String::new() }
}

// ---- an agent: a program, and the model that travels with it ------------------------------------------------------------
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentName { pub kind: String, pub model: String }
/// "codex/gpt-5.5" -> codex and gpt-5.5; "none" -> no kind.
pub fn agent_of(value: &str) -> AgentName {
    if value.is_empty() || value == NONE { return AgentName::default(); }
    match value.split_once('/') { Some((k, m)) => AgentName { kind: k.into(), model: m.into() }, None => AgentName { kind: value.into(), model: String::new() } }
}
/// An effort or an access is right or wrong for one program: "" when it fits, else why not.
pub fn fits(name: &str, value: &str, agent: &str) -> String {
    let (Some(e), Some(a)) = (entry_of(name), kind(&agent_of(agent).kind)) else { return String::new() };
    if e.ty == Ty::Effort && value != "default" && a.effort.is_empty() {
        return if a.effort_note.is_empty() { format!("{} takes no effort level when the board starts it: leave it at default.", a.name) } else { format!("{}: leave it at default.", a.effort_note) };
    }
    if e.ty == Ty::Effort && value != "default" && !a.efforts.is_empty() && !a.efforts.contains(&value) {
        return format!("{} has no effort \"{value}\": it takes {} (or default, for its own).", a.name, a.efforts.join(", "));
    }
    if e.ty == Ty::Access && value != "full" && !(value == "ask" && a.access_ask.is_some()) {
        return format!("{} cannot do \"{value}\": it never asks before it acts. Keep {name} at full, or choose another program for {}.agent.", a.name, name.split('.').next().unwrap_or(""));
    }
    String::new()
}

fn word(v: &Value) -> String { text(v).trim().to_string() }
fn check_agent(e: &Entry, v: &Value) -> Res<Value> {
    let w = word(v);
    let n = agent_of(if w.to_lowercase() == NONE { NONE } else { &w });
    if n.kind.is_empty() {
        if e.none && w.to_lowercase() == NONE { return Ok(json!(NONE)); }
        return refuse(format!("{} must be {}{}, with an optional model after a slash.", e.name, KIND_NAMES.join(", "), if e.none { " or none" } else { "" }));
    }
    let Some(k) = kind_named(&n.kind).map(|a| a.name.to_string()) else {
        let k = n.kind.to_lowercase();
        return refuse(format!("{}: no agent program \"{}\". It must be {}{}.{}", e.name, n.kind, KIND_NAMES.join(", "), if e.none { " or none" } else { "" }, hint(&k, &KIND_NAMES)));
    };
    if w.contains('/') {
        // A model with its parameters in brackets (cursor: `claude-opus-4-8[effort=high]`), or its provider after an @.
        let ok = n.model.chars().next().map(|c| c.is_ascii_alphanumeric()).unwrap_or(false) && n.model.chars().all(|c| c.is_ascii_alphanumeric() || "._:/@[]-=,".contains(c));
        if !ok { return refuse(format!("{}: \"{}\" is not a model name.", e.name, n.model)); }
    }
    if let Some(why) = refusal(&e.name, &k, !n.model.is_empty()) { return refuse(why); }
    Ok(json!(if n.model.is_empty() { k } else { format!("{k}/{}", n.model) }))
}
fn split_words(s: &str) -> Vec<String> { s.split(|c: char| c.is_whitespace() || c == ',').filter(|x| !x.is_empty()).map(String::from).collect() }
fn items_of(v: &Value) -> Option<Vec<String>> {
    match v {
        Value::Array(a) => if a.iter().all(|x| x.is_string()) { Some(a.iter().map(text).collect()) } else { None },
        Value::String(s) => Some(split_words(s)),
        _ => None,
    }
}
fn check_ty(e: &Entry, v: &Value) -> Res<Value> {
    let name = &e.name;
    match e.ty {
        Ty::Switch => {
            if let Value::Bool(b) = v { return Ok(json!(b)); }
            let w = word(v).to_lowercase();
            match (v.is_string(), w.as_str()) { (true, "true" | "yes" | "on") => Ok(json!(true)), (true, "false" | "no" | "off") => Ok(json!(false)),
                _ => refuse(format!("{name} is a switch: true or false (yes, no, on and off work too), not \"{}\".", word(v))) }
        }
        Ty::Choice => {
            let w = word(v).to_lowercase();
            if e.values.contains(&w) { Ok(json!(w)) } else { let vs: Vec<&str> = e.values.iter().map(|s| s.as_str()).collect(); refuse(format!("{name} must be {}, not \"{}\".{}", vs.join(", "), word(v), hint(&w, &vs))) }
        }
        Ty::Time => {
            let w: String = word(v).to_lowercase().split_whitespace().collect();
            let like = if text(&e.builtin) == NONE { "2h".to_string() } else { text(&e.builtin) };
            if e.none && w == NONE { return Ok(json!(NONE)); }
            let unit = if e.none { "m".to_string() } else { like[like.len() - 1..].to_string() };
            if ms(&w) == 0 {
                let meant = time_meant(&w, &unit);
                return refuse(format!("{name} is a time with a unit, such as {like} (s, m or h){}, not \"{}\".{}", if e.none { ", or none" } else { "" }, word(v),
                    if !meant.is_empty() { format!(" Did you mean {meant}?") } else if e.none { none_meant(&w) } else { String::new() }));
            }
            Ok(json!(w))
        }
        Ty::Tokens => {
            let w: String = word(v).to_lowercase().split_whitespace().collect();
            if w == NONE { return Ok(json!(NONE)); }
            let ok = split_count(&w).map(|(_, u)| u.is_empty() || u == "k" || u == "m").unwrap_or(false);
            if !ok || tokens_of(&w) < 1 {
                let meant = tokens_meant(&w);
                return refuse(format!("{name} is a count of tokens, such as 500k or 2m, or none, not \"{}\".{}", word(v), if !meant.is_empty() { format!(" Did you mean {meant}?") } else { none_meant(&w) }));
            }
            Ok(json!(tokens_say(tokens_of(&w) as f64)))
        }
        Ty::Count => {
            let n = match v { Value::Number(n) if n.as_f64().map(|f| f.fract() == 0.0).unwrap_or(false) => Some(n.as_f64().unwrap() as i64),
                Value::String(_) if !word(v).is_empty() && word(v).bytes().all(|b| b.is_ascii_digit()) => word(v).parse().ok(), _ => None };
            match n { Some(n) if n >= e.min => Ok(json!(n)), _ => refuse(format!("{name} must be a whole number, {} or more, not \"{}\".", e.min, word(v))) }
        }
        Ty::Agent => check_agent(e, v),
        Ty::Agents => {
            if let Value::String(s) = v { if s.trim().to_lowercase() == NONE { return Ok(json!(NONE)); } }
            let Some(items) = items_of(v) else { return refuse(format!("{name} is a list of agents ({}, each with an optional model after a slash), or none.", KIND_NAMES.join(", "))) };
            let names: Vec<String> = items.iter().map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
            if names.is_empty() { return refuse(format!("{name} needs a list of agents ({}, each with an optional model after a slash), or none.", KIND_NAMES.join(", "))); }
            if names.iter().any(|x| x.to_lowercase() == NONE) { return refuse(format!("{name}: none stands alone. Say none, or a list of agents without it.")); }
            let mut out: Vec<Value> = vec![];
            for x in names { let a = check_agent(e, &json!(x))?; if !out.contains(&a) { out.push(a); } }
            Ok(Value::Array(out))
        }
        Ty::Effort => {
            let w = word(v).to_lowercase();
            if w.is_empty() || !w.chars().all(|c| c.is_ascii_lowercase()) { return refuse(format!("{name} must be an effort level such as low, medium or high, or default.")); }
            Ok(json!(w))
        }
        Ty::Access => {
            let w = word(v).to_lowercase();
            if w == "full" || w == "ask" { Ok(json!(w)) } else { refuse(format!("{name} must be full or ask, not \"{}\".{}", word(v), hint(&w, &["full", "ask"]))) }
        }
        Ty::Text => {
            let Value::String(s) = v else { return refuse(format!("{name} needs words, or none.")) };
            if s.trim().is_empty() { return refuse(format!("{name} needs words, or none.")); }
            Ok(json!(if s.trim().to_lowercase() == NONE { NONE.to_string() } else { s.trim().to_string() }))
        }
        Ty::List => {
            if let Value::String(s) = v { if s.trim().to_lowercase() == NONE { return Ok(json!(NONE)); } }
            let Some(items) = items_of(v) else { return refuse(format!("{name} is a list of names, or none.")) };
            let out: Vec<String> = items.iter().map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
            Ok(if out.is_empty() { json!(NONE) } else { json!(out) })
        }
        Ty::Machines => {
            let names: Vec<String> = match v { Value::Array(a) => a.iter().map(text).collect(), _ => split_words(&word(v)) }.into_iter().map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
            if names.is_empty() { return refuse(format!("{name} must be auto, none, or machine names.")); }
            if names.len() == 1 && ["auto", NONE].contains(&names[0].to_lowercase().as_str()) { return Ok(json!(names[0].to_lowercase())); }
            let ok_name = |x: &str| { let (user, host) = match x.split_once('@') { Some((u, h)) => (Some(u), h), None => (None, x) };
                let part = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c));
                user.map(part).unwrap_or(true) && part(host) && !["auto", NONE].contains(&x.to_lowercase().as_str()) };
            if let Some(bad) = names.iter().find(|x| !ok_name(x)) { return refuse(format!("{name}: \"{bad}\" is not a machine name (host or user@host). Other values: auto, none.")); }
            Ok(json!(names.join(", ")))
        }
        Ty::Attempts => {
            let w = match v { Value::Number(_) => text(v), Value::Array(a) => a.iter().map(text).collect::<Vec<_>>().join(", "), _ => word(v) };
            if !w.is_empty() && w.bytes().all(|b| b.is_ascii_digit()) {
                let n: u64 = w.parse().unwrap_or(u64::MAX);
                if n < 1 { return refuse(format!("{name} must be 1 or more, not {w}. Did you mean 1? That is the task done once.")); }
                if n > MAX_ATTEMPTS as u64 { return refuse(format!("{name} takes {TOO_MANY}. Not {n}: did you mean {MAX_ATTEMPTS}?")); }
                return Ok(json!(n));
            }
            let names = split_words(&w);
            if names.is_empty() || w.starts_with(|c: char| c.is_ascii_digit()) || (w.starts_with('-') && w[1..].starts_with(|c: char| c.is_ascii_digit())) {
                return refuse(format!("{name} is a count (3) or a list of agents ({}), not \"{w}\".", KIND_NAMES.join(", ")));
            }
            let mut list = vec![];
            for x in names { list.push(text(&check_agent(e, &json!(x))?)); }
            if list.len() == 1 { return refuse(format!("{name}: one agent is one attempt. For that, set worker.agent {0}; for several attempts name each one ({0}, {0}) or give a count (2).", list[0])); }
            if list.len() > MAX_ATTEMPTS { return refuse(format!("{name} takes {TOO_MANY}. Not {}: did you mean {MAX_ATTEMPTS}?", list.len())); }
            Ok(json!(list.join(", ")))
        }
        Ty::Key => {
            let w = word(v);
            if w.to_lowercase() == NONE { return Ok(json!(NONE)); }
            let k = w.to_uppercase();
            let ok = v.is_string() && (2..=5).contains(&k.len()) && k.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false) && k.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
            if !ok { return refuse(format!("{name} is a short key: 2 to 5 letters or digits, the first a letter (HSW), or none; not \"{w}\".")); }
            if k == "PR" { return refuse(format!("{name} cannot be PR: on a row that already means a pull request. Choose another.")); }
            Ok(json!(k))
        }
        Ty::Notify => {
            let Value::String(s) = v else { return refuse(format!("{name} must be none, bell, or a command.")) };
            let w = s.trim();
            if w.is_empty() { return refuse(format!("{name} must be none, bell, or a command.")); }
            Ok(json!(if [NONE, "bell"].contains(&w.to_lowercase().as_str()) { w.to_lowercase() } else { w.to_string() }))
        }
    }
}
/// The value as kept, or a refusal. `agent`: the program an effort or an access is for, where it is known.
pub fn check(name: &str, value: &Value, agent: &str) -> Res<Value> {
    let e = known(name)?;
    if value.is_null() { return refuse(format!("{name} needs a value.")); }
    let v = check_ty(e, value)?;
    if !agent.is_empty() { let why = fits(name, &text(&v), agent); if !why.is_empty() { return refuse(format!("{name}: {why}")); } }
    Ok(v)
}
/// A model named in a value being set is one its program takes (agents/models.rs). Only when a value is set: a value
/// read from a file is not checked again, so reading never waits for a program's list.
pub fn models_fit(name: &str, v: &Value) -> Res<()> {
    let Some(e) = entry_of(name).filter(|e| matches!(e.ty, Ty::Agent | Ty::Agents | Ty::Attempts)) else { return Ok(()) };
    let all: Vec<String> = match v { Value::Array(a) => a.iter().map(text).collect(), _ => text(v).split(", ").map(String::from).collect() };
    for a in all.iter().map(|x| agent_of(x)) { if let Some(why) = crate::agents::models::why_not(&a.kind, &a.model) { return refuse(format!("{}: {why}", e.name)); } }
    Ok(())
}
/// A value as a person reads it.
pub fn say(v: &Value) -> String { match v { Value::Array(a) => a.iter().map(text).collect::<Vec<_>>().join(", "), _ => text(v) } }
/// A command, or nothing: `none` is a real answer that means no command.
pub fn command_of(v: &Value) -> String { let s = text(v); if s.is_empty() || s == NONE { String::new() } else { s } }
pub fn list_of(v: &Value) -> Vec<String> { strings(v) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn values() {
        assert_eq!(check("review.rounds", &json!("5"), "").unwrap(), json!(5));
        assert_eq!(check("limit.tokens", &json!("2000k"), "").unwrap(), json!("2m"));
        assert_eq!(check("limit.time", &json!("none"), "").unwrap(), json!("none"));
        assert!(check("limit.time", &json!("2 hours"), "").unwrap_err().msg.contains("Did you mean 2h?"));
        assert_eq!(check("worker.fallback", &json!("codex, claude/claude-opus-5-5"), "").unwrap(), json!(["codex", "claude/claude-opus-5-5"]));
        assert_eq!(check("attempts.by", &json!("claude, codex"), "").unwrap(), json!("claude, codex"));
        assert!(check("revew.rounds", &json!(1), "").unwrap_err().msg.contains("review.rounds"));
        assert_eq!(tokens_say(1_500_000.0), "1.5m");
        assert_eq!(tokens_say(500_000.0), "500k");
        assert_eq!(check("repo.key", &json!("hsw"), "").unwrap(), json!("HSW"));
        assert!(check("worker.access", &json!("ask"), "pi").is_err());
    }
}
