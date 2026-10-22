//! A file attached in the web board's chat box (a picture pasted, dropped or picked, or any other file), kept where
//! whoever gets the words can read it, as the herdr web UI keeps one beside its pane: the box then puts `@<path>` in
//! the message, which the agent reads the file from. Beside the Coordinator (or a heading's coordinator) in its own
//! folder, beside a task's agent in its working folder (under `.herdr-projects/files/`, which git is told to leave
//! alone), and with no Coordinator in the board's own folder (the words become a task). For another machine's agent
//! or coordinator it goes to that machine (attach/far.rs), and the path is where it landed there.
pub mod far;

use super::web::Client;
use crate::board::update::coords::{look_of, of_head, Target};
use crate::coordinator::co::Co;
use crate::flow::task::RULE_DIR;
use crate::json::*;
use crate::remote::carry::For;
use far::{Far, WebUi};
use std::path::PathBuf;

pub use crate::store::attached::MOST;
use crate::store::attached::{keep, too_big};
/// Where the file goes: a folder of this machine, or another machine.
enum Whom { Here(PathBuf), There(Far) }
/// A heading's coordinator (or another machine's top-level one): its folder here, or its machine and its pane there.
fn coordinator(c: &Client, t: &Target) -> Result<Whom, String> {
    if t.host.is_empty() {
        if !c.s.data.can_dispatch { return Err(format!("No coordinator can run here, so nothing was kept. {}", crate::coordinator::who::OFF_WORDS)); }
        return Ok(Whom::Here(crate::coordinator::state::home(Co::of(&crate::herdr::cli::current_socket(), &t.to)).join(RULE_DIR).join("files")));
    }
    let m = c.s.remotes.get(&t.host).ok_or_else(|| format!("{} is not among the machines you can reach now.", t.host))?;
    let pane = look_of(&c.s, t).map(|v| v.pane).unwrap_or_default();
    Ok(Whom::There(Far { machine: m.machine.clone(), pane, to: For::Coordinator(t.to.clone()) }))
}
/// Who the words are for (`to`: `{ id, reply, scope }` as the box says it): the task's agent or the heading's
/// coordinator with `reply`, else the Coordinator of the machine whose board showed when they were typed (`scope`; the
/// board's now when the box said none).
fn whom(c: &Client, to: &Value) -> Result<Whom, String> {
    let (id, reply) = (to.s("id"), to.b("reply") && to.b("id"));
    let scope = to.get("scope").and_then(|s| s.as_str()).map(String::from).unwrap_or_else(|| c.s.scope.clone());
    if reply {
        if let Some(t) = of_head(&c.s, id) { return coordinator(c, &t); }
        // A heading of this machine where no coordinator can run: nobody would get the words.
        if crate::board::update::heads::is_head(id) && !c.s.data.can_dispatch { return Err(format!("No coordinator can run here, so nothing was kept. {}", crate::coordinator::who::OFF_WORDS)); }
        let r = c.s.items.iter().chain(c.s.remotes.values().flat_map(|m| m.items.iter())).find(|r| r.s("id") == id).ok_or("The task those words are for is gone.")?;
        if !r.b("host") { return Ok(Whom::Here(crate::store::repos::cwd_of(r).join(RULE_DIR).join("files"))); }
        let m = c.s.remotes.get(r.s("host")).ok_or_else(|| format!("{} is not among the machines you can reach now.", r.s("host")))?;
        return Ok(Whom::There(Far { machine: m.machine.clone(), pane: r.at("link").t("pane"), to: For::Task(r.t("rid")) }));
    }
    // Another machine's board: its top-level Coordinator, as the words go (coords.rs `far_top`).
    if !scope.is_empty() && scope != "*" {
        if !c.s.remotes.contains_key(&scope) { return Err(format!("{scope} is not among the machines you can reach now.")); }
        return coordinator(c, &Target { host: scope, to: String::new(), name: String::new() });
    }
    if !c.s.data.can_dispatch { return Ok(Whom::Here(crate::store::files::data_dir().join("files"))); }
    Ok(Whom::Here(crate::coordinator::state::home(&crate::herdr::cli::current_socket()).join(RULE_DIR).join("files")))
}
/// Kept, and its path answered: `{ path }`. `args`: `{ name, type, data (base64), to }`.
pub fn attach(c: &Client, args: &Value) -> Result<Value, String> { attach_with(c, args, None) }
/// The same, with the web UI the front door reaches (far.rs) for a file that goes to another machine.
pub fn attach_with(c: &Client, args: &Value, ui: Option<&dyn WebUi>) -> Result<Value, String> {
    let whom = whom(c, args.at("to"))?;
    let data = args.s("data");
    if data.len() > MOST.div_ceil(3) * 4 + 4 { return Err(too_big(args.s("name"))); }
    let bytes = crate::util::unbase64(data).ok_or("The file could not be read.")?;
    if bytes.is_empty() { return Err("That file is empty.".into()); }
    if bytes.len() > MOST { return Err(too_big(args.s("name"))); }
    let path = match whom {
        Whom::Here(dir) => keep(&dir, args.s("name"), args.s("type"), &bytes).map_err(|e| format!("The file could not be kept: {e}"))?.display().to_string(),
        Whom::There(f) => far::send(&f, ui,args.s("name"), args.s("type"), &bytes)?,
    };
    Ok(json!({ "path": path }))
}
