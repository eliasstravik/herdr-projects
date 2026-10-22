//! Marking a repo as trusted in Codex's own config.toml, read as TOML: an entry for the repo in any quoting
//! (`[projects."…"]`, `[projects.'…']`, a dotted key) is already there, a file that does not read is never written,
//! and a file that holds the same project's table twice (what an older board wrote) is found for `doctor`.
use super::homes::codex_file;
use std::fs;
use std::path::Path;

/// What the file says about the repo: None when it has no entry; Some(trusted?) when it has one.
fn entry(conf: &toml::Table, root: &str) -> Option<bool> {
    let p = conf.get("projects")?.as_table()?.get(root)?;
    Some(p.get("trust_level").and_then(|v| v.as_str()) == Some("trusted"))
}

/// Trusted in Codex's config.toml, adding the repo's table at the end when it has none (so Codex's own comments and
/// layout stay as they are). An entry that says something else (`untrusted`) is Codex's answer and is left as it is;
/// a file that does not read, or that the table would not read in, is not touched. False: its folder dialog shows.
pub fn trust(root: &Path) -> bool {
    let root = root.display().to_string();
    let file = codex_file();
    let text = match fs::read_to_string(&file) { Ok(t) => t, Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(), Err(_) => return false };
    let Ok(conf) = text.parse::<toml::Table>() else { return false };
    if let Some(trusted) = entry(&conf, &root) { return trusted; }
    let key = toml::Value::String(root.clone()).to_string();
    let add = format!("{}\n[projects.{key}]\ntrust_level = \"trusted\"\n", if !text.is_empty() && !text.ends_with('\n') { "\n" } else { "" });
    if format!("{text}{add}").parse::<toml::Table>().ok().and_then(|c| entry(&c, &root)) != Some(true) { return false; }
    if let Some(d) = file.parent() { let _ = fs::create_dir_all(d); }
    fs::OpenOptions::new().create(true).append(true).open(&file).and_then(|mut f| std::io::Write::write_all(&mut f, add.as_bytes())).is_ok()
}

/// The projects whose `[projects.…]` table the text has more than once, in any quoting, each once.
pub fn twice(text: &str) -> Vec<String> {
    let mut seen: Vec<String> = vec![];
    let mut out: Vec<String> = vec![];
    for l in text.lines().map(str::trim).filter(|l| l.starts_with("[projects.") && !l.starts_with("[[")) {
        // The header alone is a TOML file of its own: its one key, however it is quoted.
        let Ok(t) = format!("{l}\n").parse::<toml::Table>() else { continue };
        let Some(p) = t.get("projects").and_then(|v| v.as_table()).and_then(|p| p.keys().next().cloned()) else { continue };
        if seen.contains(&p) { if !out.contains(&p) { out.push(p); } } else { seen.push(p); }
    }
    out
}

/// For `doctor`: Codex's config.toml does not read (what is wrong, what to do); None when it reads or is not there.
pub fn problem() -> Option<(String, String)> {
    let file = codex_file();
    let text = fs::read_to_string(&file).ok()?;
    let err = text.parse::<toml::Table>().err()?;
    let at = file.display();
    let doubled = twice(&text);
    if !doubled.is_empty() {
        let list = doubled.join(", ");
        return Some((format!("Codex's {at} has the table of {list} twice, so Codex does not start"), format!("Open {at} and delete the second `[projects.…]` table of {list} (its header and its trust_level line); the board does not change your file.")));
    }
    let line = err.span().map(|r| format!(" (line {})", text.get(..r.start).unwrap_or(&text).matches('\n').count() + 1)).unwrap_or_default();
    Some((format!("Codex's {at} does not read as TOML{line}, so Codex does not start: {}", err.message().trim()), format!("Open {at} and fix it{line}.")))
}
