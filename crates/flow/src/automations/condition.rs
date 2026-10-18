//! A board event's condition: `--if 'branch = feature/KIC-*'`, checked on the task the event happened to before the
//! automation fires; a mismatch is a written skip, not a firing. Reading one as it is typed, the wildcard it matches
//! with (`*` any run of characters, `?` one, the rest literal, the whole field, any case), and saying it in plain words.
//!   { field, op, pattern }   field: one of FIELDS · op: "=" or "!="
use crate::err::Res;
use crate::json::*;
use crate::store::settings::values::nearest;

/// What of the task a condition can look at.
pub const FIELDS: [&str; 3] = ["branch", "title", "repo"];
const HOW: &str = "A condition is a field, = or != and a pattern: --if 'branch = feature/KIC-*' (fields: branch, title, repo; * is anything, ? one character)";
fn unquote(s: &str) -> &str {
    let s = s.trim();
    for q in ['"', '\''] { if s.len() >= 2 && s.starts_with(q) && s.ends_with(q) { return s[1..s.len() - 1].trim(); } }
    s
}
pub fn parse_condition(typed: &str) -> Res<Value> {
    let text = unquote(typed);
    let Some(eq) = text.find('=') else { crate::fail!("\"{text}\" has no = or !=. {HOW}"); };
    let not = text[..eq].ends_with('!');
    let field = text[..eq].trim_end_matches('!').trim().to_lowercase();
    let pattern = unquote(text[eq + 1..].strip_prefix('=').unwrap_or(&text[eq + 1..]));
    if field.is_empty() { crate::fail!("\"{text}\" names no field before the {}. {HOW}", if not { "!=" } else { "=" }); }
    if !FIELDS.contains(&field.as_str()) {
        crate::fail!("\"{field}\" is not something a condition can look at.{} One of: {}.", nearest(&field, &FIELDS).map(|n| format!(" Did you mean {n}?")).unwrap_or_default(), FIELDS.join(", "));
    }
    if pattern.is_empty() { crate::fail!("The pattern after {} is empty: say what {field} is to be like (* is anything). {HOW}", if not { "!=" } else { "=" }); }
    Ok(json!({ "field": field, "op": if not { "!=" } else { "=" }, "pattern": pattern }))
}
/// Does `text` match the wildcard `pattern`, the whole of it, in any case?
pub fn like(pattern: &str, text: &str) -> bool {
    let (p, t): (Vec<char>, Vec<char>) = (pattern.to_lowercase().chars().collect(), text.to_lowercase().chars().collect());
    let (mut i, mut j, mut star, mut from) = (0, 0, None, 0);
    while j < t.len() {
        if i < p.len() && (p[i] == '?' || (p[i] != '*' && p[i] == t[j])) { i += 1; j += 1; }
        else if i < p.len() && p[i] == '*' { star = Some(i); from = j; i += 1; }
        else if let Some(s) = star { i = s + 1; from += 1; j = from; }
        else { return false; }
    }
    p[i..].iter().all(|c| *c == '*')
}
/// Why a trigger's condition keeps it from firing for this task, or None when it fires (or has no condition).
pub fn mismatch(trigger: &Value, task: &Obj) -> Option<String> {
    let c = trigger.at("if");
    if !c.b("field") { return None; }
    let value = task.t(c.s("field"));
    if like(c.s("pattern"), &value) == (c.s("op") != "!=") { return None; }
    Some(format!("its condition did not match ({})", if value.is_empty() { format!("no {} yet", c.s("field")) } else { format!("{} \"{value}\"", c.s("field")) }))
}
/// In plain words, after the trigger's: "if its branch is like *KIC-*".
pub fn say_condition(c: &Value) -> String { format!("if its {} is {}like {}", c.s("field"), if c.s("op") == "!=" { "not " } else { "" }, c.s("pattern")) }
/// --if as given: None when it is not, Some(None) for `--if none` (no condition), else the condition.
pub fn condition_of(flags: &Obj) -> Res<Option<Option<Value>>> {
    match flags.get("if") {
        None | Some(Value::Bool(false)) => Ok(None),
        Some(Value::Bool(true)) => crate::fail!("--if needs its condition. {HOW}, or --if none to remove it."),
        Some(v) if text(v).trim().eq_ignore_ascii_case("none") => Ok(Some(None)),
        Some(v) => parse_condition(&text(v)).map(|c| Some(Some(c))),
    }
}
/// The trigger with this condition, or with none. Refused on a schedule: it has no task to look at.
pub fn with_condition(trigger: &Value, c: Option<&Value>) -> Res<Value> {
    if trigger.s("kind") != "event" { crate::fail!("A condition (--if) is for a trigger of --when: a schedule has no task to look at."); }
    let mut t = trigger.clone();
    let o = t.as_object_mut().expect("a trigger is an object");
    match c { Some(c) => { o.put("if", c.clone()); } None => { o.shift_remove("if"); } }
    Ok(t)
}
