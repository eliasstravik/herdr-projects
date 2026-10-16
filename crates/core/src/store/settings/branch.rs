//! The task branch's name (`workspace.branch`): a pattern with `{n}`, `{slug}` and `{key}` in it, checked by git's own
//! `check-ref-format` with sample values put in, refused with the nearest pattern git takes, and filled in for a task.
use super::values::nearest;
use crate::err::{Error, Res};
use crate::json::*;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::Mutex;

pub const PLACEHOLDERS: [&str; 3] = ["n", "slug", "key"];
/// What a pattern is checked with: a task's number, its title as a slug, its repo's key and number.
const SAMPLE: [(&str, &str); 3] = [("n", "123"), ("slug", "fix-the-login"), ("key", "ABC-123")];
static SAID: Mutex<Option<HashMap<String, bool>>> = Mutex::new(None);

/// A branch name git takes (`git check-ref-format --branch`). Asked again for every read of a setting, so each answer
/// is kept: git runs once per name.
pub fn git_takes(name: &str) -> bool {
    if let Some(&ok) = SAID.lock().unwrap().as_ref().and_then(|m| m.get(name)) { return ok; }
    let ok = !name.is_empty() && Command::new("git").args(["check-ref-format", "--branch", name]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .status().map(|s| s.success()).unwrap_or(false);
    SAID.lock().unwrap().get_or_insert_with(HashMap::new).insert(name.to_string(), ok);
    ok
}
enum Part { Words(String), Hole(String) }
/// A pattern in its words and its placeholders, or why it cannot be read.
fn parts(pattern: &str) -> Result<Vec<Part>, String> {
    let mut out = vec![];
    let mut rest = pattern;
    while let Some(open) = rest.find(['{', '}']) {
        if rest[open..].starts_with('}') { return Err("has a } with no { before it".into()); }
        if open > 0 { out.push(Part::Words(rest[..open].into())); }
        let Some(close) = rest[open..].find('}') else { return Err("has a { that is never closed".into()) };
        out.push(Part::Hole(rest[open + 1..open + close].into()));
        rest = &rest[open + close + 1..];
    }
    if !rest.is_empty() { out.push(Part::Words(rest.into())); }
    Ok(out)
}
/// The pattern with each placeholder filled in. A placeholder that comes out empty (a title with no letters) takes one
/// separator next to it along, so `work/{n}-{slug}` is `work/7`, never `work/7-`.
fn fill(parts: &[Part], value: impl Fn(&str) -> String) -> String {
    let mut out = String::new();
    let mut drop_next = false;
    for p in parts {
        match p {
            Part::Words(w) => { let w = if drop_next { w.strip_prefix(['-', '_', '.', '/']).unwrap_or(w) } else { w }; out.push_str(w); drop_next = false; }
            Part::Hole(h) => {
                let v = value(h);
                if v.is_empty() { if out.ends_with(['-', '_', '.', '/']) { out.pop(); } else { drop_next = true; } } else { out.push_str(&v); }
            }
        }
    }
    out
}
fn sample(parts: &[Part]) -> String { fill(parts, |h| SAMPLE.iter().find(|(k, _)| *k == h).map(|(_, v)| v.to_string()).unwrap_or_default()) }
/// The placeholder a wrong one most likely meant: the nearest, or the one it starts as (`{number}`, `{k}`).
fn meant(h: &str) -> Option<String> {
    let low = h.to_lowercase();
    nearest(h, &PLACEHOLDERS).or_else(|| PLACEHOLDERS.iter().find(|p| !low.is_empty() && (low.starts_with(*p) || p.starts_with(&low))).map(|p| p.to_string()))
}
/// A pattern read however it is written: a `}` with no `{` left out, a `{` never closed closed at the end (or left
/// out, when another `{` follows), a wrong placeholder the one it meant (or left out).
fn loose(pattern: &str) -> Vec<Part> {
    let (mut rest, mut open) = (String::new(), None);
    for c in pattern.chars() {
        match c {
            '{' => { if let Some(i) = open { rest.remove(i); } open = Some(rest.len()); rest.push(c); }
            '}' => if open.take().is_some() { rest.push(c); },
            _ => rest.push(c),
        }
    }
    if open.is_some() { rest.push('}'); }
    let mut out = vec![];
    for p in parts(&rest).unwrap_or_else(|_| vec![Part::Words(rest.replace(['{', '}'], ""))]) {
        match p { Part::Hole(h) if !PLACEHOLDERS.contains(&h.as_str()) => { if let Some(m) = meant(&h) { out.push(Part::Hole(m)); } } p => out.push(p) }
    }
    out
}
/// The nearest pattern git would take: what it refuses in the words between the placeholders turned into `-` or left
/// out, and a number added when there is none. The built-in one when nothing nearer is taken.
fn mended(pattern: &str) -> String {
    let mut s = String::new();
    for p in loose(pattern) {
        match p {
            Part::Hole(h) => { s.push('{'); s.push_str(&h); s.push('}'); }
            Part::Words(w) => for c in w.chars() { s.push(if c.is_whitespace() || c.is_control() || "~^:?*[\\".contains(c) { '-' } else { c }); },
        }
    }
    let mut s = crate::re!(r"-{2,}").replace_all(&s, "-").into_owned();
    s = crate::re!(r"\.{2,}").replace_all(&s, ".").into_owned();
    s = crate::re!(r"/{2,}").replace_all(&s, "/").into_owned();
    s = s.replace("@{", "{");
    let comp = |c: &str| { let mut c = c.trim_start_matches('.'); while let Some(x) = c.strip_suffix(".lock") { c = x.trim_end_matches('.'); } c.to_string() };
    let comps: Vec<String> = s.split('/').map(comp).filter(|c| !c.is_empty()).collect();
    let mut s = comps.join("/").trim_start_matches(['-', '/']).trim_end_matches(['.', '/']).to_string();
    if !s.contains("{n}") && !s.contains("{key}") { s = if s.is_empty() { "work/{n}".into() } else { format!("{s}-{{n}}") }; }
    match parts(&s) { Ok(ps) if git_takes(&sample(&ps)) => s, _ => BUILT_IN.into() }
}
const BUILT_IN: &str = "work/{n}-{slug}";
fn refuse<T>(msg: String) -> Res<T> { Err(Error::refused(msg)) }
/// A pattern as kept, or refused with the nearest one git takes.
pub fn check(name: &str, v: &Value) -> Res<Value> {
    let Value::String(s) = v else { return refuse(format!("{name} is a branch name with {{n}}, {{slug}} or {{key}} in it, such as {BUILT_IN}.")) };
    let w = s.trim();
    let near = || format!(" Did you mean {}?", mended(w));
    let ps = match parts(w) { Ok(p) => p, Err(why) => return refuse(format!("{name}: \"{w}\" {why}.{}", near())) };
    if let Some(h) = ps.iter().find_map(|p| match p { Part::Hole(h) if !PLACEHOLDERS.contains(&h.as_str()) => Some(h), _ => None }) {
        return refuse(format!("{name}: no placeholder {{{h}}}. It takes {{n}} (the task's number), {{slug}} (its title) and {{key}} (its repo's key and number, ABC-123).{}", near()));
    }
    let numbered = ps.iter().any(|p| matches!(p, Part::Hole(h) if h == "n" || h == "key"));
    if numbered && git_takes(&sample(&ps)) { return Ok(json!(w)); }
    if !numbered { return refuse(format!("{name} needs {{n}} or {{key}} in it, so that each task's branch is its own, not \"{w}\".{}", near())); }
    refuse(format!("{name}: \"{w}\" is not a branch name git takes (it would make {}).{}", sample(&ps), near()))
}
/// A task's branch from the pattern: its number (an attempt's letter included: `7b`), its title as a slug, its repo's
/// key with the number (`ABC-7b`; the number alone in a repo with no key). "" when the pattern cannot be read.
pub fn branch_of(pattern: &str, n: &str, slug: &str, key: &str) -> String {
    let Ok(ps) = parts(pattern) else { return String::new() };
    let keyed = if key.is_empty() { n.to_string() } else { format!("{key}-{n}") };
    fill(&ps, |h| match h { "n" => n.to_string(), "slug" => slug.to_string(), "key" => keyed.clone(), _ => String::new() })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patterns() {
        assert_eq!(check("workspace.branch", &json!(" feat/{key}-{slug} ")).unwrap(), json!("feat/{key}-{slug}"));
        assert!(check("workspace.branch", &json!("work/{num}")).unwrap_err().msg.contains("Did you mean work/{n}?"));
        assert!(check("workspace.branch", &json!("my work/{n}..{slug}")).unwrap_err().msg.contains("Did you mean my-work/{n}.{slug}?"));
        assert!(check("workspace.branch", &json!("work/{slug}")).unwrap_err().msg.contains("Did you mean work/{slug}-{n}?"));
        assert!(check("workspace.branch", &json!("work/{n")).unwrap_err().msg.contains("Did you mean work/{n}?"));
        assert!(check("workspace.branch", &json!("work/{n.lock.lock")).unwrap_err().msg.contains("Did you mean work/{n}?"));
        assert!(check("workspace.branch", &json!("work/{n}.lock.lock")).unwrap_err().msg.contains("Did you mean work/{n}?"));
        assert!(check("workspace.branch", &json!("}w{{ork/{n}/{slug")).unwrap_err().msg.contains("Did you mean work/{n}/{slug}?"));
        assert!(check("workspace.branch", &json!("@.lock")).unwrap_err().msg.contains("Did you mean @-{n}?"));
        // Every refusal names a pattern that is taken.
        for bad in ["work/{n}.lock.lock", "{n", "}", "{", "{}", "{{n}}", "-/{n}/.", "..{n}..", "a/.lock/{n}", "@{n}", "/{n}//", "x~^:?*[\\{n}", "{slug}.lock", "..", "@", "-", "work/{n}.", "{x}{y}", "a b c"] {
            let Err(e) = check("workspace.branch", &json!(bad)) else { continue };
            let near = e.msg.rsplit_once("Did you mean ").and_then(|(_, n)| n.strip_suffix('?')).unwrap_or_else(|| panic!("{bad}: {}", e.msg));
            assert!(check("workspace.branch", &json!(near)).is_ok(), "{bad} -> {near}");
        }
        assert_eq!(branch_of("work/{n}-{slug}", "7", "", ""), "work/7");
        assert_eq!(branch_of("{slug}/{key}", "7b", "", "ABC"), "ABC-7b");
        assert_eq!(branch_of("u/{key}/{slug}", "7", "x", ""), "u/7/x");
    }
}
