//! The words on the command line, and the flags each command takes. A command refuses a flag it does not know and
//! names the nearest right one: a mistyped flag (`list --archvied`) used to be kept and silently ignored, and the
//! command did something else than what was asked.
use crate::err::Res;
use crate::json::*;
use crate::store::settings::values::{known, nearest};
use crate::store::settings_list::SETTINGS;

/// Positional arguments, flags (`--flag value`, `--flag` true, `--no-flag` false), and each flag as it was typed.
pub struct Args { pub pos: Vec<String>, pub flags: Obj, pub typed: Obj }
const BOOL: [&str; 17] = ["json", "archived", "all", "done", "delegate", "force", "help", "skill", "research", "chat", "every-repo", "anyway", "now", "clear", "machine", "main-coordinator", "machine-coordinator"];
pub fn parse(argv: &[String]) -> Args {
    let (mut pos, mut flags, mut typed) = (vec![], Obj::new(), Obj::new());
    let mut i = 0;
    while i < argv.len() {
        let a = &argv[i];
        if a == "--" { pos.extend(argv[i + 1..].iter().cloned()); break; }
        if a.starts_with("--") && a.len() > 2 {
            let eq = a.find('=');
            let name = match eq { Some(e) => &a[2..e], None => &a[2..] };
            let shown = name.strip_prefix("no-").filter(|_| eq.is_none()).unwrap_or(name);
            typed.put(shown, match eq { Some(e) => &a[..e], None => a.as_str() });
            if let Some(e) = eq { flags.put(name, &a[e + 1..]); }
            else if let Some(n) = name.strip_prefix("no-") { flags.put(n, false); }
            else if BOOL.contains(&name) || i + 1 >= argv.len() || argv[i + 1].starts_with("--") { flags.put(name, true); }
            else { i += 1; flags.put(name, argv[i].as_str()); }
        } else { pos.push(a.clone()); }
        i += 1;
    }
    Args { pos, flags, typed }
}

/// Taken by every command.
const ALWAYS: [&str; 2] = ["json", "help"];
const SETTING: &str = "any work setting by its name (--review.when never)";
pub struct Takes { pub flags: &'static [&'static str], pub settings: bool }
const fn f(flags: &'static [&'static str]) -> Takes { Takes { flags, settings: false } }
const fn s(flags: &'static [&'static str]) -> Takes { Takes { flags, settings: true } }
const ADD: &[&str] = &["desc", "description", "repo", "delegate", "research", "chat", "after", "on"];
const LIST: &[&str] = &["archived", "all", "query"];
const WHERE: &[&str] = &["repo", "task", "automation", "main-coordinator", "machine-coordinator", "project-coordinator"];
const AUTO_ADD: &[&str] = &["prompt", "desc", "schedule", "when", "every-repo", "if", "repo", "research", "chat"];
const AUTO_CHANGE: &[&str] = &["name", "prompt", "schedule", "when", "every-repo", "if", "research", "chat"];
/// One command per line, beside the commands it belongs with, never at the end. A command with subcommands names
/// what each takes instead (`automations add`), and the command's own for one that is not named. The entries herdr,
/// an agent's hook and the board itself run are here too: no command lets a flag through unread.
fn takes(cmd: &str, sub: &str) -> Option<Takes> {
    Some(match cmd {
        "board" | "--skill" | "skill" => f(&["skill"]),
        "help" | "hook" | "keeper" | "keeper-work" | "open" | "open-coordinator" | "approve-current" | "serve" | "remote" => f(&[]),
        "docs" => f(&["check", "where", "guard"]),
        "turn-ended" => f(&["id", "role", "token", "round"]),
        "plan-written" => f(&["id", "token"]),
        "run" => f(&["force", "merged"]),
        "add" | "new" => s(ADD),
        "list" | "ls" => f(LIST),
        "report" => f(&["state", "kind", "msg", "title", "progress", "eta", "desc", "repo"]),
        "describe" | "rename" | "tell" | "stop" | "start" | "pick" | "review" | "retry" | "fix" | "finish" | "details" | "pin" | "clear" => f(&[]),
        "restore" | "unarchive" => f(&["repo"]),
        "archive" => f(&["done", "repo"]),
        "delegate" => s(&["research", "chat", "after", "now"]),
        "after" => f(&["clear"]),
        "approve" => f(&["anyway"]),
        "set" | "unset" | "show" => f(WHERE),
        "models" => f(&[]),
        "automations" | "automation" => match sub { "add" | "new" => s(AUTO_ADD), "change" | "edit" => f(AUTO_CHANGE), _ => f(&[]) },
        "autopilot" => match sub { "on" => s(&["machine", "project", "goal"]), "off" => f(&["machine", "project", "all"]), "show" => f(&["machine", "project"]), _ => f(&[]) },
        "coordinator" => match sub { "tell" => f(&["delegate", "machine", "project"]), "new" | "fresh" => f(&["agent", "effort", "machine", "project"]), _ => f(&["machine", "project"]) },
        "sweep" => f(&["dry"]),
        "machines" | "web-ui" | "remove-extras" | "reopen" | "version" | "install" | "update" | "doctor" => f(&[]),
        "voice" => f(&["from"]),
        "housekeep" => f(&["pr"]),
        "check" => f(&["all", "fresh"]),
        _ => return None,
    })
}
fn has_subs(cmd: &str) -> bool { ["automations", "automation", "coordinator", "autopilot"].contains(&cmd) }
/// The flags of a command that it does not know, as typed. A name with a dot is a setting where settings are taken.
pub fn unknown_flags(cmd: &str, flags: &Obj, typed: &Obj, sub: &str) -> Vec<String> {
    let Some(t) = takes(cmd, sub) else { return vec![] };
    flags.keys().filter(|k| !ALWAYS.contains(&k.as_str()) && !t.flags.contains(&k.as_str()) && !(t.settings && k.contains('.')))
        .map(|k| typed.get(k).map(text).filter(|s| !s.is_empty()).unwrap_or_else(|| format!("--{k}"))).collect()
}
/// Refuses the first flag the command does not know: what it is, the nearest right one, and all it takes. And for a
/// setting no setting has that name: the same refusal `set` gives, with the nearest name.
pub fn refuse_unknown(cmd: &str, a: &Args) -> Res<()> {
    let sub = a.pos.get(1).map(|s| s.as_str()).unwrap_or("");
    let Some(t) = takes(cmd, sub) else { return Ok(()) };
    let odd = unknown_flags(cmd, &a.flags, &a.typed, sub);
    if odd.is_empty() { if t.settings { for k in a.flags.keys() { if k.contains('.') { known(k)?; } } } return Ok(()); }
    let all: Vec<&str> = t.flags.iter().chain(ALWAYS.iter()).copied().collect();
    let first = odd[0].trim_start_matches("--");
    let word = first.strip_prefix("no-").unwrap_or(first);
    let names: Vec<&str> = if t.settings { SETTINGS.iter().map(|s| s.name.as_str()).collect() } else { vec![] };
    let who = if cmd == "board" { "herdr-projects".to_string() } else if has_subs(cmd) && !sub.is_empty() { format!("{cmd} {sub}") } else { cmd.to_string() };
    let listed = all.iter().map(|f| format!("--{f}")).collect::<Vec<_>>().join(", ");
    // A setting's name where no setting is taken: said as that, with the name it most likely was.
    if word.contains('.') && !t.settings {
        let every: Vec<&str> = SETTINGS.iter().map(|s| s.name.as_str()).collect();
        let meant = nearest(word, &every).filter(|m| m != word).map(|m| format!(" (and the setting is named {m})")).unwrap_or_default();
        crate::fail!("{who} does not know {}: it takes no settings{meant}. It takes: {listed}. Nothing was done.", odd.join(", "));
    }
    // `--reviewer claude` meant a setting of that area: the first one that starts with the word, else the nearest name.
    let pool: Vec<&str> = all.iter().chain(names.iter()).copied().collect();
    let near = names.iter().find(|n| n.starts_with(&format!("{word}."))).map(|s| s.to_string()).or_else(|| nearest(word, &pool));
    let list = if t.settings { format!("{listed}, {SETTING}") } else { listed };
    crate::fail!("{who} does not know {}.{} It takes: {list}. Nothing was done.", odd.join(", "), near.map(|n| format!(" Did you mean --{n}?")).unwrap_or_default());
}
/// The options of add and delegate that are the task's own settings: every --<setting> (a name with a dot), --chat /
/// --no-chat, the short form of chat.only, and --research / --no-research, the short form of research.only (given
/// apart). Any other option was refused before.
pub fn pick_choices(flags: &Obj) -> (Vec<(String, Value)>, Option<bool>) {
    let mut set: Vec<(String, Value)> = flags.iter().filter(|(k, _)| k.contains('.')).map(|(k, v)| (k.clone(), v.clone())).collect();
    if let Some(v) = flags.get("chat") { set.push((crate::chat_only::CHAT.into(), json!(*v == json!(true)))); }
    (set, flags.get("research").map(|v| *v == json!(true)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(a: &[&str]) -> Args { parse(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>()) }
    #[test]
    fn unknown_flags_are_refused_with_the_nearest() {
        let a = args(&["list", "--archvied"]);
        let e = refuse_unknown("list", &a).unwrap_err().msg;
        assert!(e.contains("does not know --archvied. Did you mean --archived?"), "{e}");
        let a = args(&["add", "x", "--no-reserch"]);
        assert!(refuse_unknown("add", &a).unwrap_err().msg.contains("--no-reserch"));
        let a = args(&["add", "x", "--research", "--worker.agent", "codex"]);
        assert!(refuse_unknown("add", &a).is_ok());
        assert_eq!(a.flags.at("worker.agent"), &json!("codex"));
        let a = args(&["tell", "x/1", "--review.when", "never"]);
        assert!(refuse_unknown("tell", &a).unwrap_err().msg.contains("takes no settings"));
    }
}
