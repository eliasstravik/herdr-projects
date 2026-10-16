//! A file attached in the web board's chat box (a picture pasted, dropped or picked, or any other file), kept where
//! whoever gets the words can read it, as the herdr web UI keeps one beside its pane: the box then puts `@<path>` in
//! the message, which the agent reads the file from. Beside the Coordinator in its own folder, beside a task's agent in
//! its working folder (under `.herdr-projects/files/`, which git is told to leave alone), and with no Coordinator in
//! the board's own folder (the words become a task). Only on this machine: another machine's gets none.
use super::web::Client;
use crate::flow::task::RULE_DIR;
use crate::json::*;
use std::path::{Path, PathBuf};

/// The web UI's own limit for one file (its shared/attachments.ts), checked by the page before it sends any of it.
pub const MOST: usize = 8 * 1024 * 1024;
/// The pictures the web UI stores as a paste under a name of its own; any other file keeps its own name.
const PICTURES: [(&str, &str); 4] = [("image/png", "png"), ("image/jpeg", "jpg"), ("image/gif", "gif"), ("image/webp", "webp")];

/// A file's own name made safe to keep: letters, digits, `.`, `_` and `-`, its last 80; the end after its last dot.
fn own_name(name: &str) -> (String, String) {
    let clean: String = name.chars().map(|c| if c.is_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect();
    let clean = clean.trim_start_matches(['.', '_']);
    let clean: String = clean.chars().rev().take(80).collect::<Vec<_>>().into_iter().rev().collect();
    let (base, end) = match clean.rfind('.') { Some(d) if d > 0 => (&clean[..d], clean[d + 1..].to_lowercase()), _ => (clean.as_str(), String::new()) };
    let end = if !end.is_empty() && end.len() <= 10 && end.chars().all(|c| c.is_ascii_alphanumeric()) { end } else { "bin".into() };
    (if base.is_empty() { "file".into() } else { base.into() }, end)
}
/// Where the words' reader can read a file: who they are for (`to`: the task's agent with `reply`, else the
/// Coordinator), on the machine whose board showed when they were typed (`scope`).
fn folder(c: &Client, to: &Value) -> Result<PathBuf, String> {
    let (id, reply, scope) = (to.s("id"), to.b("reply") && to.b("id"), to.s("scope"));
    if !scope.is_empty() && scope != "*" { return Err(format!("A file goes only to this machine's Coordinator and agents, not to {scope}'s.")); }
    if reply {
        let r = c.s.items.iter().find(|r| r.s("id") == id).ok_or("The task those words are for is gone.")?;
        if r.b("host") { return Err(format!("A file goes only to this machine's agents, not to {}'s.", r.s("host"))); }
        return Ok(crate::store::repos::cwd_of(r).join(RULE_DIR).join("files"));
    }
    if !c.s.data.can_dispatch { return Ok(crate::store::files::data_dir().join("files")); }
    Ok(crate::coordinator::state::home(&crate::herdr::cli::current_socket()).join(RULE_DIR).join("files"))
}
/// Kept, and its path answered: `{ path }`. `args`: `{ name, type, data (base64), to }`.
pub fn attach(c: &Client, args: &Value) -> Result<Value, String> {
    let dir = folder(c, args.at("to"))?;
    let data = args.s("data");
    if data.len() > MOST.div_ceil(3) * 4 + 4 { return Err(too_big(args.s("name"))); }
    let bytes = crate::util::unbase64(data).ok_or("The file could not be read.")?;
    if bytes.is_empty() { return Err("That file is empty.".into()); }
    if bytes.len() > MOST { return Err(too_big(args.s("name"))); }
    let path = keep(&dir, args.s("name"), args.s("type"), &bytes).map_err(|e| format!("The file could not be kept: {e}"))?;
    Ok(json!({ "path": path.display().to_string() }))
}
fn too_big(name: &str) -> String { format!("Too large to attach: {}. A file can be up to 8 MB.", if name.is_empty() { "that file" } else { name }) }
/// Written once, readable only by you, under a name no other file has. The folder's own `.gitignore` keeps it out of
/// git, also in a repo that does not leave `.herdr-projects/` alone.
pub fn keep(dir: &Path, name: &str, kind: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
    use std::io::Write;
    use crate::os::{no_follow, private};
    std::fs::create_dir_all(dir)?;
    // A link in the folder's place (`.herdr-projects` or `files` in a working folder an agent writes in) would send
    // the file somewhere else: refused, and the `.gitignore` is never written through one.
    for d in [dir, dir.parent().unwrap_or(dir)] { if std::fs::symlink_metadata(d)?.file_type().is_symlink() { return Err(std::io::Error::other("its folder is a link")); } }
    match no_follow(std::fs::OpenOptions::new().write(true).create_new(true)).open(dir.join(".gitignore")) {
        Ok(mut f) => f.write_all(b"*\n")?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    let stamp = chrono::DateTime::from_timestamp_millis(crate::clock::now()).map(|d| d.format("%Y%m%d-%H%M%S").to_string()).unwrap_or_default();
    let unique = format!("{stamp}-{}", crate::util::random_hex(4));
    let file = match PICTURES.iter().find(|(k, _)| *k == kind) { Some((_, end)) => format!("paste-{unique}.{end}"), None => { let (base, end) = own_name(name); format!("{base}-{unique}.{end}") } };
    let path = dir.join(file);
    no_follow(private(std::fs::OpenOptions::new().write(true).create_new(true))).open(&path)?.write_all(bytes)?;
    Ok(path)
}
