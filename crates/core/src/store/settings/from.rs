//! Where a worktree starts (`workspace.from`): `main`, `here` or the name of a branch at the remote, checked by git's
//! own `check-ref-format` and refused with the nearest name it takes.
use super::branch::git_takes;
use super::values::nearest;
use crate::err::{Error, Res};
use crate::json::*;

pub const WORDS: [&str; 2] = ["main", "here"];
fn refuse<T>(msg: String) -> Res<T> { Err(Error::refused(msg)) }
/// A branch named by its name alone: `origin/develop` and `refs/heads/develop` are `develop`.
fn bare(w: &str) -> &str { w.strip_prefix("refs/heads/").or_else(|| w.strip_prefix("refs/remotes/origin/")).or_else(|| w.strip_prefix("origin/")).unwrap_or(w) }
/// The nearest name git takes: what it refuses turned into `-`, dots and slashes doubled or at the ends taken off.
fn mended(w: &str) -> String {
    let s: String = bare(w).chars().map(|c| if c.is_whitespace() || c.is_control() || "~^:?*[\\".contains(c) { '-' } else { c }).collect();
    let s = crate::re!(r"-{2,}").replace_all(&s, "-").into_owned();
    let s = crate::re!(r"\.{2,}").replace_all(&s, ".").into_owned();
    let s = crate::re!(r"/{2,}").replace_all(&s, "/").into_owned().replace("@{", "");
    let comps: Vec<String> = s.split('/').map(|c| { let mut c = c.trim_start_matches('.'); while let Some(x) = c.strip_suffix(".lock") { c = x; } c.trim_end_matches('.').to_string() }).filter(|c| !c.is_empty()).collect();
    comps.join("/").trim_start_matches('-').to_string()
}
/// `main`, `here` or a branch's name as kept, or refused with the nearest one.
pub fn check(name: &str, v: &Value) -> Res<Value> {
    let Value::String(s) = v else { return refuse(format!("{name} is main, here or the name of a branch at the remote, such as develop.")) };
    let w = s.trim();
    if WORDS.contains(&w.to_lowercase().as_str()) { return Ok(json!(w.to_lowercase())); }
    if w.is_empty() { return refuse(format!("{name} is main, here or the name of a branch at the remote, such as develop.")); }
    if w == "HEAD" || w.to_lowercase() == "origin/head" { return refuse(format!("{name}: \"{w}\" is no branch. Did you mean here (the commit your checkout is on) or main (the remote's main branch)?")); }
    if bare(w) != w && !bare(w).is_empty() && git_takes(bare(w)) { return refuse(format!("{name} is a branch's name at the remote, without where it is: not \"{w}\". Did you mean {}?", bare(w))); }
    if !w.starts_with('-') && !w.contains("@{") && git_takes(w) { return Ok(json!(w)); }
    let near = mended(w);
    let hint = if !near.is_empty() && git_takes(&near) { format!(" Did you mean {near}?") } else { nearest(w, &WORDS).map(|n| format!(" Did you mean {n}?")).unwrap_or_default() };
    refuse(format!("{name}: \"{w}\" is not a branch name git takes. It is main, here or the name of a branch at the remote, such as develop.{hint}"))
}
/// The branch a value names, when it names one: not for `main` or `here`.
pub fn named(value: &str) -> Option<&str> { (!value.is_empty() && !WORDS.contains(&value)).then_some(value) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn values() {
        let ok = |v: &str| check("workspace.from", &json!(v)).unwrap();
        let bad = |v: &str| check("workspace.from", &json!(v)).unwrap_err().msg;
        assert_eq!((ok(" Main "), ok("here"), ok("develop"), ok("release/2.0")), (json!("main"), json!("here"), json!("develop"), json!("release/2.0")));
        assert!(bad("origin/develop").contains("Did you mean develop?"), "{}", bad("origin/develop"));
        assert!(bad("refs/heads/develop").contains("Did you mean develop?"));
        assert!(bad("my branch").contains("Did you mean my-branch?"), "{}", bad("my branch"));
        assert!(bad("release..2.lock").contains("Did you mean release.2?"), "{}", bad("release..2.lock"));
        assert!(bad("HEAD").contains("Did you mean here"));
        assert!(bad("-x").contains("Did you mean x?"), "{}", bad("-x"));
        assert_eq!((named("main"), named("here"), named("develop")), (None, None, Some("develop")));
    }
}
