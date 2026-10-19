//! herdr's config.toml: the marked blocks the plugin adds on a yes (the key that opens the board, the sidebar rows),
//! checked on a copy first, backed up, and put back when herdr rejects them. herdr is told to reload after each change.
use super::{extras, save};
use super::found::{bound_key, has_sidebar};
use crate::flow::tabs::AGENT_TAB;
use crate::herdr::{cli::launch, keys};
use crate::json::*;
use crate::runner::inline;
use crate::store::{files::{data_dir, with_lock_wait}, marks::tilde, tasks::one_line};
use std::path::PathBuf;

/// Where herdr's config is (verified on 0.9.3: $HERDR_CONFIG_PATH, else <XDG config>/herdr/config.toml).
pub fn herdr_config() -> PathBuf {
    if let Ok(p) = std::env::var("HERDR_CONFIG_PATH") { return p.into(); }
    let base = std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|_| crate::store::files::user_home().join(".config"));
    base.join("herdr").join("config.toml")
}
fn begin(what: &str) -> String { format!("# >>> herdr-projects {what} (added by the herdr-projects plugin; `herdr-projects remove-extras` removes it)") }
fn end(what: &str) -> String { format!("# <<< herdr-projects {what}") }
fn block(what: &str, arg: &str) -> String {
    match what {
        "key" => format!("[[keys.command]]\nkey = \"{arg}\"\ntype = \"plugin_action\"\ncommand = \"herdr-projects.open\"\ndescription = \"work board\""),
        // Row 1: herdr's dot, the machine where there is more than one, and the space's name, which for a task is its
        // number and title (flow/sidebar.rs); the tab's name too, but not a task's own tabs. Row 2: who is on it and
        // where it stands. Row 3: what it says, its percent and time left.
        _ => format!("[ui.sidebar.agents]\nrows = [[\"state_icon\", \"machine\", \"$task\", {{ token = \"workspace\", rules = [{{ equals = \"Mission Control\", hide = true }}] }}, {{ token = \"tab\", rules = [{{ starts_with = \"{AGENT_TAB}\", hide = true }}, {{ starts_with = \"Reviewer #\", hide = true }}] }}], [\"agent\"], [{{ token = \"$summary\", dim = true }}, \"$progress\"]]\n\n[ui.sidebar.spaces]\nrows = [[\"state_icon\", \"workspace\"], [\"branch\", \"git_status\"], [{{ token = \"$summary\", dim = true }}, \"$progress\"]]"),
    }
}
pub fn block_text(what: &str, arg: &str) -> String { format!("{}\n{}\n{}", begin(what), block(what, arg), end(what)) }
fn read() -> String { std::fs::read_to_string(herdr_config()).unwrap_or_default() }
fn is_link() -> bool { std::fs::symlink_metadata(herdr_config()).map(|m| m.file_type().is_symlink()).unwrap_or(false) }
fn herdr_out(a: &[&str], env: Option<(&str, String)>) -> Option<(i32, String)> {
    if let Some(f) = crate::herdr::cli::fake() { let r = f.run(&a.iter().map(|x| x.to_string()).collect::<Vec<_>>(), ""); return Some((r.status, format!("{}{}", r.stdout, if r.stdout.is_empty() { r.stderr } else { String::new() }))); }
    let mut c = std::process::Command::new(crate::herdr::cli::bin());
    c.args(a).stdin(std::process::Stdio::null());
    if let Some((k, v)) = env { c.env(k, v); }
    let child = c.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().ok()?;
    let r = crate::herdr::cli::wait_with_timeout(child, 15000).ok()?;
    Some((r.status, format!("{}{}", r.stdout, if r.stdout.is_empty() { r.stderr } else { String::new() })))
}
/// What the first run can offer, given what herdr's config already holds.
pub fn offers() -> Value {
    let text = read();
    let x = extras();
    let bound = regex::Regex::new(r#"command\s*=\s*["']herdr-projects\.open["']"#).unwrap().is_match(&text);
    let offer_key = !bound && x.s("key") != "no"; // an existing binding is left as it is
    let (key, note) = if offer_key && !x.b("asked") { keys::suggest(&text, &keys::defaults()) } else { (String::new(), String::new()) };
    // Rows of the user's own, in any spelling, are kept: ours added beside them would make herdr reject the file.
    let (own, ours) = (has_sidebar(&text), text.contains(&begin("sidebar")));
    let sidebar = !own && x.s("sidebarAnswer") != "no";
    let kept = own && !ours && x.s("sidebarAnswer") != "no";
    json!({ "file": herdr_config().display().to_string(), "asked": x.b("asked"), "key": if offer_key { json!({ "suggest": key, "note": note }) } else { Value::Null }, "sidebar": if sidebar { json!({}) } else { Value::Null }, "sidebarKept": if kept { json!(kept_note()) } else { Value::Null } })
}
/// Said once when the config has sidebar rows of its own: they are kept, and ours are there to put in by hand.
fn kept_note() -> String {
    format!("Your own sidebar rows in {} are kept. To show each task's stage there too, merge these into them yourself, then run: herdr server reload-config\n\n{}", tilde(&herdr_config().display().to_string()), block("sidebar", ""))
}
/// The key that opens the board, as herdr's config binds it ("prefix+w"), or "" when none is bound.
pub fn board_key() -> String { bound_key(&read()) }
/// That key as key presses, for the board in herdr's popup, which herdr hands every key to: its own key closes it.
pub fn board_presses() -> Option<(Vec<String>, String)> { keys::presses(&board_key(), &read(), &keys::built_in()) }
/// May the board take this key? Asked as soon as one is typed at the first run, and again before it is written.
pub fn check_key(key: &str) -> Result<(), String> { keys::check(key, &read(), &keys::defaults(), &tilde(&herdr_config().display().to_string())) }
/// Would herdr take this config? Asked of a copy, never of the real file. Yes when herdr cannot be asked.
fn accepted(text: &str) -> Result<(), String> {
    let cand = data_dir().join(format!("config-candidate-{}.toml", std::process::id()));
    let _ = std::fs::create_dir_all(data_dir());
    if std::fs::write(&cand, text).is_err() { return Ok(()); }
    let r = herdr_out(&["config", "check"], Some(("HERDR_CONFIG_PATH", cand.display().to_string())));
    let _ = std::fs::remove_file(&cand);
    match r { None => Ok(()), Some((0, out)) if out.to_lowercase().contains("ok") => Ok(()), Some((_, out)) => Err(one_line(&out, 300)) }
}
fn reload() -> (String, String) {
    let Some((code, out)) = herdr_out(&["server", "reload-config"], None) else { return ("unknown".into(), "herdr did not answer".into()) };
    match serde_json::from_str::<Value>(&out) {
        Ok(j) => { let r = j.at("result"); (r.t("status"), strings(r.at("diagnostics")).join("; ")) }
        Err(_) => (if code == 0 { "applied".into() } else { "unknown".into() }, out.trim().to_string()),
    }
}
/// Add one marked block. The edit is checked on a copy first, the file is backed up, and herdr is told to reload; if
/// herdr rejects the result the backup comes back and herdr reloads again. Ok(message) or Err((message, paste)).
pub fn add_block(what: &str, arg: &str) -> Result<String, (String, Option<String>)> {
    let file = herdr_config();
    let text = read();
    if text.contains(&begin(what)) { return Ok("already there".into()); } // never added twice
    let arg = if what == "key" { arg.trim().to_lowercase() } else { arg.to_string() };
    let b = block_text(what, &arg);
    if what == "key" { keys::check(&arg, &text, &keys::defaults(), &tilde(&file.display().to_string())).map_err(|m| (m, None))?; } // never a key in use
    let next = format!("{text}{}\n{b}\n", if !text.is_empty() && !text.ends_with('\n') { "\n" } else { "" });
    let paste = format!("Could not change {}. Add this to it yourself, then run: herdr server reload-config\n\n{b}", tilde(&file.display().to_string()));
    accepted(&next).map_err(|why| (format!("herdr would reject that: {why}"), None))?;
    let backup = PathBuf::from(format!("{}.bak-herdr-projects", file.display()));
    let existed = file.exists();
    if existed && is_link() { return Err((paste, Some(b))); } // a managed link: not ours to rewrite
    let wrote = (|| -> std::io::Result<()> { if let Some(d) = file.parent() { std::fs::create_dir_all(d)?; } if existed { std::fs::copy(&file, &backup)?; } std::fs::write(&file, &next) })();
    if wrote.is_err() { return Err((paste, Some(b))); }
    let (status, why) = reload();
    let bad = status == "failed" || (status == "partial" && what == "key" && why.to_lowercase().contains(&arg));
    if bad {
        if existed { let _ = std::fs::copy(&backup, &file); } else { let _ = std::fs::remove_file(&file); }
        reload();
        return Err((format!("herdr rejected the change, so your settings were put back as they were. It said: {}", one_line(&why, 300)), None));
    }
    if what == "sidebar" { save(json!({ "sidebar": true })); }
    Ok(if status == "unknown" { format!("Added. herdr could not be asked to reload ({}): restart herdr or run herdr server reload-config.", one_line(&why, 120)) } else { "Added.".into() })
}
/// The sidebar rows the plugin wrote, brought up to this version's: an older block is replaced by the rows above,
/// between the same marker lines. Only the plugin's own block, and only when it differs; checked on a copy first,
/// and put back if herdr rejects it. At herdr's start and when the board opens.
pub fn refresh_sidebar() -> bool {
    if inline() && std::env::var("HERDR_CONFIG_PATH").is_err() { return false; } // tests never touch the real config
    let (file, text, want) = (herdr_config(), read(), block_text("sidebar", ""));
    let Some(at) = text.find(&begin("sidebar")) else { return false };
    let Some(e) = text[at..].find(&end("sidebar")).map(|i| at + i + end("sidebar").len()) else { return false };
    if text[at..e] == want || is_link() || !file.exists() { return false; }
    with_lock_wait("extras", 15000, || {
        let next = format!("{}{want}{}", &text[..at], &text[e..]);
        if accepted(&next).is_err() { return Ok(false); }
        if std::fs::copy(&file, format!("{}.bak-herdr-projects", file.display())).is_err() || std::fs::write(&file, &next).is_err() { return Ok(false); }
        let r = launch(None, &["server".into(), "reload-config".into()], "", 15000).map(|r| r.stdout).unwrap_or_default();
        if regex::Regex::new(r#""status":\s*"failed""#).unwrap().is_match(&r) { let _ = std::fs::write(&file, &text); let _ = launch(None, &["server".into(), "reload-config".into()], "", 15000); return Ok(false); }
        Ok(true)
    }).unwrap_or(false)
}
/// An earlier version offered to turn on herdr's notification pop-ups. The plugin raises none now, so the block it
/// wrote for that is taken out again, at herdr's start and when the board opens.
pub fn remove_popups() -> bool {
    if inline() && std::env::var("HERDR_CONFIG_PATH").is_err() { return false; } // tests never touch the real config
    if is_link() || !herdr_config().exists() { return false; }
    with_lock_wait("extras", 15000, || Ok(remove_blocks(Some("popups")))).unwrap_or(false)
}
/// The whole file made `next`, for what an older install left in it (take_over/herdr_config.rs): checked on a copy
/// first, backed up, herdr told to reload, and put back as it was when herdr rejects it.
pub fn rewrite(file: &std::path::Path, next: &str) -> Result<(), String> {
    if std::fs::symlink_metadata(file).map(|m| m.file_type().is_symlink()).unwrap_or(false) { return Err(format!("{} is a link, not ours to rewrite", tilde(&file.display().to_string()))); }
    accepted(next).map_err(|why| format!("herdr would reject the change: {why}"))?;
    let text = std::fs::read_to_string(file).unwrap_or_default();
    std::fs::copy(file, format!("{}.bak-herdr-projects", file.display())).and_then(|_| std::fs::write(file, next)).map_err(|e| e.to_string())?;
    let (status, why) = reload();
    if status == "failed" { let _ = std::fs::write(file, &text); reload(); return Err(format!("herdr rejected the change, so it was put back. It said: {}", one_line(&why, 300))); }
    Ok(())
}
/// Take out the plugin's marked blocks (one kind, or every kind).
pub fn remove_blocks(what: Option<&str>) -> bool {
    let file = herdr_config();
    let text = read();
    let w = what.map(regex::escape).unwrap_or_else(|| "[^\\n]*".into());
    if !regex::Regex::new(&format!("(?m)^# >>> herdr-projects {w}")).unwrap().is_match(&text) { return false; }
    let blocks = regex::Regex::new(&format!(r"\n?# >>> herdr-projects {w}[^\n]*\n(?s:.*?)# <<< herdr-projects {w}[^\n]*\n?")).unwrap();
    let next = blocks.replace_all(&text, "\n").to_string();
    let next = crate::re!(r"\n{3,}").replace_all(&next, "\n\n").to_string();
    if std::fs::copy(&file, format!("{}.bak-herdr-projects", file.display())).is_err() || std::fs::write(&file, next).is_err() { return false; }
    reload();
    true
}
