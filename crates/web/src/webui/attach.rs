//! A file attached in the web board's chat box (a picture pasted, dropped or picked, or any other file), kept where
//! whoever gets the words can read it, as the herdr web UI keeps one beside its pane: the box then puts `@<path>` in
//! the message, which the agent reads the file from. Beside the Coordinator in its own folder, beside a task's agent in
//! its working folder (under `.herdr-projects/files/`, which git is told to leave alone), and with no Coordinator in
//! the board's own folder (the words become a task). Only on this machine: another machine's gets none.
use super::web::Client;
use crate::flow::task::RULE_DIR;
use crate::json::*;
use std::path::PathBuf;

pub use crate::store::attached::MOST;
use crate::store::attached::{keep, too_big};
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
