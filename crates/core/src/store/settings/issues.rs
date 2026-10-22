//! The GitHub issues a task's work closes (`merge.closes`): issue numbers of the task's own repo, as typed (`2203,
//! #2204`) and as kept (a list of numbers), or `none`.
use super::values::NONE;
use crate::err::{Error, Res};
use crate::json::*;

/// The value as kept: the numbers in the order given, each once, or `none`.
pub fn check(name: &str, v: &Value) -> Res<Value> {
    let bad = |w: &str, why: &str| Err(Error::refused(format!("{name} is a list of issue numbers of the task's own repo (2203, 2204), or none; not \"{w}\".{why}")));
    let words: Vec<String> = match v {
        Value::Number(_) => vec![text(v)],
        Value::Array(a) => a.iter().map(text).collect(),
        Value::String(s) => s.split(|c: char| c.is_whitespace() || c == ',').filter(|x| !x.is_empty()).map(String::from).collect(),
        _ => return bad(&text(v), ""),
    };
    if words.len() == 1 && words[0].to_lowercase() == NONE { return Ok(json!(NONE)); }
    if words.is_empty() { return bad("", ""); }
    let mut out: Vec<u64> = vec![];
    for w in &words {
        let digits = w.strip_prefix('#').unwrap_or(w);
        match digits.parse::<u64>() {
            Ok(n) if n > 0 && digits.bytes().all(|b| b.is_ascii_digit()) => { if !out.contains(&n) { out.push(n); } }
            _ if w.contains('/') => return bad(w, " An issue of another repo is not closed for now: name the task's own by its number."),
            _ => return bad(w, ""),
        }
    }
    Ok(json!(out))
}
/// The numbers of a kept value; none for `none` or nothing.
pub fn numbers(v: &Value) -> Vec<u64> {
    match v { Value::Array(a) => a.iter().filter_map(|x| x.as_u64().or_else(|| text(x).trim_start_matches('#').parse().ok())).filter(|n| *n > 0).collect(), _ => vec![] }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn issues() {
        assert_eq!(check("merge.closes", &json!("2203, #2204,2203")).unwrap(), json!([2203, 2204]));
        assert_eq!(check("merge.closes", &json!(7)).unwrap(), json!([7]));
        assert_eq!(check("merge.closes", &json!("none")).unwrap(), json!("none"));
        assert!(check("merge.closes", &json!("o/r#5")).unwrap_err().msg.contains("another repo"));
        assert!(check("merge.closes", &json!("12a")).is_err());
        assert_eq!(numbers(&json!([3, 4])), vec![3, 4]);
        assert!(numbers(&json!("none")).is_empty());
    }
}
